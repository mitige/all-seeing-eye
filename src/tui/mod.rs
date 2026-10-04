//! Dashboard TUI live (Task 11) — l'« All-Seeing Eye ».
//!
//! La boucle ne lance PAS le pipeline : le CLI (Task 12) fait tourner
//! le pipeline dans un thread et pousse les [`Event`] dans un canal ;
//! [`run`] les consomme. [`browse`] reconstruit l'état depuis un
//! rapport existant (navigation seule).

pub mod dashboard;
mod detail;
mod keys;
mod tree;

use anyhow::{Context, Result};
use crossterm::event::{self, Event as CrosstermEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use dashboard::{App, StepStatus};
use keys::Action;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::engine::events::Event;
use crate::report::Report;

/// Cadence de la boucle : poll des touches (spinner et chrono
/// rafraîchis à chaque tour).
const POLL: Duration = Duration::from_millis(50);

/// Bornes du drain par tick : au plus `DRAIN_MAX_EVENTS` events OU
/// `DRAIN_MAX_DUREE` écoulée. Le reliquat attend le tick suivant —
/// le canal conserve tout, rien n'est perdu, le clavier ne starve pas.
const DRAIN_MAX_EVENTS: usize = 1000;
const DRAIN_MAX_DUREE: Duration = Duration::from_millis(5);

/// Lance le TUI : consomme les events du pipeline (qui tourne dans un
/// autre thread, branché par le CLI Task 12), restaure le terminal à
/// la sortie (`q`), retourne le rapport final.
///
/// Erreur si le terminal ne s'initialise pas, si l'IO lâche en cours
/// de route, ou si l'utilisateur quitte avant `RunFinished` (pas de
/// rapport à retourner).
pub fn run(rx: mpsc::Receiver<Event>, battery_name: String, started: Instant) -> Result<Report> {
    let (_guard, mut terminal) = TerminalGuard::enter()?;
    let mut app = App::new(battery_name, started);
    boucle(&mut terminal, &mut app, Some(&rx))?;
    app.finished()
        .cloned()
        .context("run quitté avant RunFinished : aucun rapport")
}

/// Mode navigation seule (`all-seeing-eye report`) : reconstruit
/// l'état depuis un rapport existant — post-run d'emblée, pas de
/// canal d'events.
pub fn browse(report: Report) -> Result<()> {
    let (_guard, mut terminal) = TerminalGuard::enter()?;
    let mut app = App::from_report(report);
    boucle(&mut terminal, &mut app, None)
}

/// RAII : le terminal est restauré au drop — raw mode off, écran
/// alternatif quitté — même en cas de panic (unwind) : jamais de
/// shell laissé dans un état bancal.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<(Self, Terminal<CrosstermBackend<io::Stdout>>)> {
        enable_raw_mode().context("activation du raw mode")?;
        // Dès que le raw mode est actif, le guard existe : une erreur
        // d'init ultérieure restaure quand même le terminal.
        let guard = Self;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("entrée dans l'écran alternatif")?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout)).context("init terminal")?;
        Ok((guard, terminal))
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Boucle principale : touches (cadence [`POLL`]) → drain des events
/// pipeline → tick spinner → redraw si dirty. Retourne sur `q` (ou
/// Ctrl+C) ; le terminal est restauré par le [`TerminalGuard`].
fn boucle(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: Option<&mpsc::Receiver<Event>>,
) -> Result<()> {
    let mut dirty = true; // premier rendu d'office
    loop {
        // 1. Touches : TOUTES celles en attente — une seule par tour
        //    laisserait une rafale clavier s'accumuler (starvation).
        //    Resize & co : dirty quand même, ratatui recalcule au
        //    redraw.
        if event::poll(POLL).context("poll terminal")? {
            loop {
                if let CrosstermEvent::Key(key) = event::read().context("lecture event terminal")? {
                    match keys::action_for(key) {
                        Some(Action::Quit) => return Ok(()),
                        Some(Action::Next) => app.next(),
                        Some(Action::Prev) => app.prev(),
                        Some(Action::ScrollUp) => app.scroll_up(),
                        Some(Action::ScrollDown) => app.scroll_down(),
                        Some(Action::ToggleFailures) => app.toggle_failures(),
                        Some(Action::Export) => {
                            // Le succès pose son propre message (chemin) ;
                            // l'erreur doit être visible aussi.
                            if let Err(e) = app.export() {
                                app.set_footer_message(format!("export impossible : {e:#}"));
                            }
                        }
                        None => {}
                    }
                }
                if !event::poll(Duration::ZERO).context("poll terminal")? {
                    break;
                }
            }
            dirty = true;
        }
        // 2. Events du pipeline : drain borné — et plus rien n'est
        //    attendu d'un pipeline mort (canal fermé sans verdict).
        if let Some(rx) = rx {
            if !app.pipeline_dead() && drain(rx, app) {
                dirty = true;
            }
        }
        // 3. Tick : spinner + chrono vivants en cours de run — figés
        //    si le pipeline est mort (pas de spinner éternel). Le TTL
        //    du footer tourne dans tous les cas : un message qui
        //    expire impose un redraw.
        let en_run = app.finished().is_none() && !app.pipeline_dead();
        let expire = app.tick();
        if en_run || expire {
            dirty = true;
        }
        // 4. Redraw si dirty.
        if dirty {
            terminal
                .draw(|f| {
                    let area = f.area();
                    render(f, app, area);
                })
                .context("rendu terminal")?;
            dirty = false;
        }
    }
}

/// Draine les events en attente du canal dans l'app, borné à
/// [`DRAIN_MAX_EVENTS`] events ou [`DRAIN_MAX_DUREE`] par tick (le
/// reliquat reste dans le canal pour le tick suivant). Un canal fermé
/// SANS `RunFinished` marque le pipeline mort — état visible, on
/// cesse d'attendre. True si un redraw est nécessaire.
fn drain(rx: &mpsc::Receiver<Event>, app: &mut App) -> bool {
    drain_borne(rx, app, DRAIN_MAX_EVENTS, DRAIN_MAX_DUREE)
}

/// Corps de [`drain`], bornes paramétrées : les tests injectent une
/// borne temporelle très large pour valider la borne en NOMBRE
/// d'events sans dépendre du timing wall — sous charge, 5 ms peuvent
/// s'écouler avant les 1000 events et rendaient le test flaky.
fn drain_borne(
    rx: &mpsc::Receiver<Event>,
    app: &mut App,
    max_events: usize,
    max_duree: Duration,
) -> bool {
    let debut = Instant::now();
    let mut recu = false;
    let mut n = 0;
    loop {
        match rx.try_recv() {
            Ok(ev) => {
                app.apply(ev);
                recu = true;
                n += 1;
            }
            Err(mpsc::TryRecvError::Empty) => break,
            // Le sender est tombé sans verdict : plus rien n'arrivera.
            // (Après RunFinished, la fermeture est la fin normale.)
            Err(mpsc::TryRecvError::Disconnected) => {
                if app.finished().is_none() && !app.pipeline_dead() {
                    app.set_pipeline_dead();
                    recu = true; // redraw : le bandeau apparaît
                }
                break;
            }
        }
        if n >= max_events || debut.elapsed() >= max_duree {
            break;
        }
    }
    recu
}

/// Chrono « mm:ss » (les minutes débordent au-delà d'une heure —
/// format compact, jamais de panic).
fn mmss(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Cadre complet : header (3 lignes), centre (arbre 30 % | détail
/// 70 %), footer (1 ligne).
fn render(f: &mut Frame, app: &App, area: Rect) {
    let [haut, centre, bas] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    let [gauche, droite] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(centre);
    render_header(f, app, haut);
    tree::render(f, app, gauche);
    detail::render(f, app, droite);
    render_footer(f, app, bas);
}

/// Header : `all-seeing-eye ▸ batterie ▸ étape courante + spinner ▸
/// chrono mm:ss`. Post-run : score global + durée du run (figée, lue
/// dans le rapport).
fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let titre = Span::styled(
        "all-seeing-eye",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let mut spans = vec![titre, Span::raw(format!(" ▸ {} ▸ ", app.battery()))];
    if let Some(report) = app.finished() {
        spans.push(Span::styled(
            format!("score global {:.1}%", report.scores.global),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(
            " ▸ {}",
            mmss(Duration::from_secs_f64(report.duration_secs.max(0.0)))
        )));
    } else if app.pipeline_dead() {
        // Canal fermé sans verdict : on le hurle, le chrono continue.
        spans.push(Span::styled(
            "pipeline interrompu",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" ▸ {}", mmss(app.started().elapsed()))));
    } else {
        match app.current_step() {
            Some(step) => {
                let st = app.step(step);
                let (icone, couleur) = match st.status {
                    StepStatus::Running => (app.spinner_char().to_string(), Color::Yellow),
                    StepStatus::Ok => ("✓".to_string(), Color::Green),
                    StepStatus::Ko => ("✗".to_string(), Color::Red),
                    StepStatus::Skipped => ("⤼".to_string(), Color::DarkGray),
                    StepStatus::Pending => ("·".to_string(), Color::DarkGray),
                };
                spans.push(Span::raw(format!("{} ", st.label)));
                spans.push(Span::styled(icone, Style::default().fg(couleur)));
            }
            None => spans.push(Span::styled(
                "en attente",
                Style::default().fg(Color::DarkGray),
            )),
        }
        spans.push(Span::raw(format!(" ▸ {}", mmss(app.started().elapsed()))));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

/// Footer : pipeline mort (bandeau rouge dédié, prioritaire), sinon
/// message éphémère (export) s'il y en a un, sinon les raccourcis —
/// avec rappel du filtre échecs actif.
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let ligne = if app.pipeline_dead() && app.finished().is_none() {
        Line::styled(
            "pipeline interrompu sans verdict — q pour quitter",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else if let Some(msg) = app.footer_message() {
        Line::styled(msg, Style::default().fg(Color::Green))
    } else {
        let mut texte = "↑↓/jk naviguer · f échecs · e exporter · q quitter".to_string();
        if app.filter_failures() {
            texte.push_str("  [échecs]");
        }
        Line::styled(texte, Style::default().fg(Color::DarkGray))
    };
    f.render_widget(Paragraph::new(ligne), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::{Event, Step};
    use crate::report::{Report, Scores};
    use dashboard::App;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn mini_report() -> Report {
        Report {
            project: "mini".to_string(),
            steps: Vec::new(),
            tests: Vec::new(),
            norme: Vec::new(),
            scores: Scores {
                global: 66.7,
                ..Scores::default()
            },
            duration_secs: 1.0,
        }
    }

    #[test]
    fn mmss_formate_le_chrono() {
        assert_eq!(mmss(Duration::from_secs(0)), "00:00");
        assert_eq!(mmss(Duration::from_secs(5)), "00:05");
        assert_eq!(mmss(Duration::from_secs(65)), "01:05");
        // Au-delà d'une heure : les minutes débordent, jamais de panic.
        assert_eq!(mmss(Duration::from_secs(3600)), "60:00");
    }

    #[test]
    fn deconnexion_sans_verdict_marque_le_pipeline_mort() {
        let (tx, rx) = mpsc::channel::<Event>();
        let mut app = App::new("mini".to_string(), Instant::now());
        tx.send(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        })
        .unwrap();
        // Le thread pipeline meurt (panic…) sans envoyer RunFinished.
        drop(tx);
        assert!(drain(&rx, &mut app));
        assert!(
            app.pipeline_dead(),
            "la déconnexion sans verdict doit être un état visible"
        );
        // Bandeau dédié : l'utilisateur sait que le spinner ne rendra
        // jamais de verdict.
        let s = texte(&app, 100, 30);
        assert!(
            s.contains("pipeline interrompu sans verdict"),
            "bandeau pipeline mort absent\n{s}"
        );
    }

    #[test]
    fn deconnexion_apres_run_finished_n_est_pas_une_mort() {
        let (tx, rx) = mpsc::channel();
        let mut app = App::new("mini".to_string(), Instant::now());
        tx.send(Event::RunFinished {
            report: Box::new(mini_report()),
        })
        .unwrap();
        // Fin normale : le CLI ferme le canal après le verdict.
        drop(tx);
        assert!(drain(&rx, &mut app));
        assert!(!app.pipeline_dead(), "le verdict est là : pas de mort");
    }

    #[test]
    fn drain_est_borne_et_laisse_le_reliquat_au_tick_suivant() {
        let (tx, rx) = mpsc::channel();
        let mut app = App::new("mini".to_string(), Instant::now());
        // 10k events d'un coup (rafale) : les checks ne sont pas
        // cappés, ils comptent exactement les events appliqués.
        for i in 0..10_000 {
            tx.send(Event::CheckFinished {
                step: Step::Build,
                name: format!("check {i}"),
                ok: true,
                detail: String::new(),
            })
            .unwrap();
        }
        // Bornes injectées : 1000 events, temps très large — le test
        // valide la borne en NOMBRE d'events, jamais le timing wall
        // (la borne de 5 ms de production le rendait flaky sous
        // charge : le reliquat variait selon l'ordonnancement).
        let drain_1000 = |rx: &mpsc::Receiver<Event>, app: &mut App| {
            drain_borne(rx, app, 1000, Duration::from_secs(60))
        };
        assert!(drain_1000(&rx, &mut app));
        assert_eq!(
            app.step(Step::Build).checks.len(),
            1000,
            "le drain doit sortir après 1000 events (clavier non affamé)"
        );
        // Reliquat intact : le tick suivant en absorbe 1000 de plus.
        assert!(drain_1000(&rx, &mut app));
        assert_eq!(app.step(Step::Build).checks.len(), 2000);
        // …et les ticks d'après absorbent tout, rien n'est perdu.
        for _ in 0..8 {
            assert!(drain_1000(&rx, &mut app));
        }
        assert_eq!(app.step(Step::Build).checks.len(), 10_000);
    }

    #[test]
    fn drain_applique_tout_ce_qui_est_en_attente() {
        let (tx, rx) = mpsc::channel();
        let mut app = App::new("mini".to_string(), Instant::now());
        // Canal vide : rien appliqué, pas de redraw.
        assert!(!drain(&rx, &mut app));
        tx.send(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        })
        .unwrap();
        tx.send(Event::LogLine {
            step: Step::Build,
            line: "cc main.c".to_string(),
        })
        .unwrap();
        tx.send(Event::RunFinished {
            report: Box::new(mini_report()),
        })
        .unwrap();
        drop(tx);
        assert!(drain(&rx, &mut app));
        assert_eq!(app.step(Step::Build).logs, vec!["cc main.c".to_string()]);
        assert!(app.finished().is_some());
        // Tout est consommé.
        assert!(!drain(&rx, &mut app));
    }

    /// Rend le dashboard complet (header + arbre + détail + footer)
    /// dans un TestBackend et retourne le texte affiché.
    fn texte(app: &App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render(f, app, area);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn rendu_complet_header_et_footer_en_cours_de_run() {
        let mut app = App::new("mini".to_string(), Instant::now());
        app.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        let s = texte(&app, 100, 30);
        assert!(s.contains("all-seeing-eye"), "header absent\n{s}");
        assert!(s.contains("mini"), "batterie absente\n{s}");
        assert!(s.contains("Compilation"), "étape courante absente\n{s}");
        assert!(
            s.contains("↑↓/jk naviguer · f échecs · e exporter · q quitter"),
            "footer absent\n{s}"
        );
    }

    #[test]
    fn rendu_post_run_montre_le_score_global() {
        let mut app = App::new("mini".to_string(), Instant::now());
        app.apply(Event::RunFinished {
            report: Box::new(mini_report()),
        });
        let s = texte(&app, 100, 30);
        assert!(s.contains("66.7"), "score global absent\n{s}");
    }

    #[test]
    fn footer_montre_le_message_export() {
        let mut app = App::new("mini".to_string(), Instant::now());
        app.apply(Event::RunFinished {
            report: Box::new(mini_report()),
        });
        app.set_footer_message("exporté → /tmp/x".to_string());
        let s = texte(&app, 100, 30);
        assert!(s.contains("exporté → /tmp/x"), "message footer absent\n{s}");
    }

    #[test]
    fn petit_terminal_ne_panique_pas() {
        let app = App::new("mini".to_string(), Instant::now());
        let _ = texte(&app, 10, 3);
        let _ = texte(&app, 0, 0);
    }
}
