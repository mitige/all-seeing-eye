//! Panneau droit du dashboard : détail de la sélection (Task 11).
//!
//! Suit [`App::selected`] : Step → résumé + checks épinglés en haut,
//! logs en dessous (collés en bas par défaut, `scroll_back` remonte) ;
//! Test → verdict + diff coloré (`-` rouge, `+` vert, contexte gris) ;
//! Fault → `file:line:col severity message (rule)`. L'étape Verdict
//! post-run affiche le détail des scores. Le rendu lit [`App`] sans
//! jamais le muter.

use crate::engine::events::{Step, TestVerdict};
use crate::tui::dashboard::{couleur_severite, App, Selection, StepStatus};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Rend le détail de la sélection dans `area`.
pub(crate) fn render(f: &mut Frame, app: &App, area: Rect) {
    let (titre, epingle, scrolle) = contenu(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" détail — {titre} "));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return; // terminal minuscule : dégradé, jamais panic
    }
    // Checks/résumé épinglés en haut, logs scrollables en dessous —
    // sauf si la place manque : tout concaténé, scrollé en bas.
    let h_pin = epingle.len() as u16;
    if h_pin > 0 && inner.height > h_pin + 1 {
        let [haut, bas] =
            Layout::vertical([Constraint::Length(h_pin), Constraint::Min(0)]).areas(inner);
        f.render_widget(Paragraph::new(epingle), haut);
        rend_scrolle(f, scrolle, app, bas);
    } else {
        let tout: Vec<Line> = epingle.into_iter().chain(scrolle).collect();
        rend_scrolle(f, tout, app, inner);
    }
}

/// Rend les `lignes` ancrées en bas : `scroll_back` lignes de recul
/// depuis la fin (0 = suit les derniers logs). Tant que `scroll_back`
/// est ouvert, la fenêtre est gelée sur la longueur au moment du
/// scroll : les nouveaux logs ne font plus dériver la vue.
fn rend_scrolle(f: &mut Frame, lignes: Vec<Line>, app: &App, area: Rect) {
    let h = area.height as usize;
    let len = app.gel_logs(lignes.len());
    let end = len.saturating_sub(app.scroll_back());
    let start = end.saturating_sub(h);
    f.render_widget(Paragraph::new(lignes[start..end].to_vec()), area);
}

/// Contenu du panneau : (titre, lignes épinglées, lignes scrollables).
fn contenu(app: &App) -> (String, Vec<Line<'static>>, Vec<Line<'static>>) {
    match app.selected() {
        Selection::Step(step) => contenu_step(app, step),
        Selection::Test { group, index } => contenu_test(app, group, index),
        Selection::Fault(i) => contenu_fault(app, i),
    }
}

/// Détail d'une étape : résumé + checks épinglés, logs scrollables.
/// L'étape Verdict post-run affiche le détail des scores du rapport.
fn contenu_step(app: &App, step: Step) -> (String, Vec<Line<'static>>, Vec<Line<'static>>) {
    let st = app.step(step);
    let titre = st.label.clone();
    // Verdict post-run : le détail des scores EST le verdict.
    if step == Step::Verdict {
        if let Some(report) = app.finished() {
            let mut lignes: Vec<Line<'static>> = report
                .scores
                .par_groupe
                .iter()
                .map(|(g, s)| Line::raw(format!("{g} … {s:.1}%")))
                .collect();
            lignes.push(Line::styled(
                format!("SCORE GLOBAL : {:.1}%", report.scores.global),
                Style::default().fg(Color::Cyan),
            ));
            return (titre, lignes, Vec::new());
        }
    }
    let mut epingle: Vec<Line<'static>> = Vec::new();
    if let Some(summary) = &st.summary {
        let couleur = match st.status {
            StepStatus::Ok => Color::Green,
            StepStatus::Ko => Color::Red,
            _ => Color::Gray,
        };
        epingle.push(Line::styled(summary.clone(), Style::default().fg(couleur)));
    }
    for (name, ok, detail) in &st.checks {
        let (icone, couleur) = if *ok {
            ("✓", Color::Green)
        } else {
            ("✗", Color::Red)
        };
        let mut texte = format!("{icone} {name}");
        if !detail.is_empty() {
            texte.push_str(&format!(" — {detail}"));
        }
        let style = Style::default().fg(couleur);
        // Un détail peut être multi-lignes (stderr d'un pre_command,
        // extrait de build…) : une `Line` ratatui n'éclate pas les
        // '\n' — première ligne avec le check, suite en continuation
        // indentée, jamais de texte mangé.
        let mut lignes = texte.split('\n');
        if let Some(premiere) = lignes.next() {
            epingle.push(Line::styled(premiere.to_string(), style));
        }
        for suite in lignes {
            epingle.push(Line::styled(format!("    {suite}"), style));
        }
    }
    // Fautes cappées (MAX_NORME) : le reliquat est compté, jamais muet.
    if step == Step::Norme && app.norme_tronquees() > 0 {
        epingle.push(Line::styled(
            format!("… et {} autres", app.norme_tronquees()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let scrolle: Vec<Line<'static>> = st.logs.iter().map(|l| Line::raw(l.clone())).collect();
    if epingle.is_empty() && scrolle.is_empty() {
        epingle.push(Line::styled(
            "en attente…".to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    (titre, epingle, scrolle)
}

/// Détail d'un test : verdict épinglé, diff coloré en dessous.
fn contenu_test(
    app: &App,
    group: usize,
    index: usize,
) -> (String, Vec<Line<'static>>, Vec<Line<'static>>) {
    let t = &app.groups()[group].tests[index];
    let (ligne_verdict, diff) = match &t.verdict {
        None => (
            Line::from(vec![
                Span::styled(
                    app.spinner_char().to_string(),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" en cours…"),
            ]),
            Vec::new(),
        ),
        Some(TestVerdict::Passed) => (
            Line::styled("✓ passed", Style::default().fg(Color::Green)),
            Vec::new(),
        ),
        Some(TestVerdict::Failed { diff, .. }) => (
            Line::styled("✗ failed", Style::default().fg(Color::Red)),
            diff.lines().map(ligne_diff).collect(),
        ),
        Some(TestVerdict::Crashed(_)) => (
            Line::styled(
                format!(
                    "💥 crash ({})",
                    t.detail.as_deref().unwrap_or("signal inconnu")
                ),
                Style::default().fg(Color::Red),
            ),
            Vec::new(),
        ),
        Some(TestVerdict::Timeout) => (
            Line::styled(
                format!(
                    "⏱ timeout ({})",
                    t.detail.as_deref().unwrap_or("délai dépassé")
                ),
                Style::default().fg(Color::Yellow),
            ),
            Vec::new(),
        ),
    };
    (t.name.clone(), vec![ligne_verdict], diff)
}

/// Une ligne de diff colorée : `-` rouge, `+` vert, contexte gris.
fn ligne_diff(l: &str) -> Line<'static> {
    let couleur = if l.starts_with('-') {
        Color::Red
    } else if l.starts_with('+') {
        Color::Green
    } else {
        Color::Gray
    };
    Line::styled(l.to_string(), Style::default().fg(couleur))
}

/// Détail d'une faute de norme : `file:line:col severity message
/// (rule)`, coloré par sévérité.
fn contenu_fault(app: &App, i: usize) -> (String, Vec<Line<'static>>, Vec<Line<'static>>) {
    let faute = &app.norme()[i];
    let titre = format!("{}:{}", faute.file.display(), faute.line);
    let ligne = Line::styled(
        format!(
            "{}:{}:{} {} {} ({})",
            faute.file.display(),
            faute.line,
            faute.col,
            faute.severity.label(),
            faute.message,
            faute.rule
        ),
        Style::default().fg(couleur_severite(&faute.severity)),
    );
    (titre, vec![ligne], Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::{Event, Step, TestVerdict};
    use crate::norme::{NormeFault, Severity};
    use crate::tui::dashboard::{App, Selection};
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;
    use std::time::Instant;

    fn app() -> App {
        App::new("mini".to_string(), Instant::now())
    }

    fn texte(app: &App, w: u16, h: u16) -> (String, TestBackend) {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render(f, app, area);
            })
            .unwrap();
        let backend = terminal.backend();
        let buf = backend.buffer();
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        (s, backend.clone())
    }

    fn va_a(app: &mut App, cible: Selection) {
        let mut garde = 0;
        while app.selected() != cible {
            app.next();
            garde += 1;
            assert!(garde < 200, "sélection {cible:?} introuvable");
        }
    }

    #[test]
    fn detail_etape_montre_checks_puis_logs_en_bas_par_defaut() {
        let mut app = app();
        app.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        app.apply(Event::CheckFinished {
            step: Step::Build,
            name: "make".to_string(),
            ok: true,
            detail: String::new(),
        });
        for i in 0..30 {
            app.apply(Event::LogLine {
                step: Step::Build,
                line: format!("l{i:02}"),
            });
        }
        va_a(&mut app, Selection::Step(Step::Build));
        let (s, _) = texte(&app, 60, 12);
        assert!(s.contains("make"), "check absent\n{s}");
        // Collé en bas par défaut : la dernière ligne de log est visible.
        assert!(s.contains("l29"), "dernier log absent\n{s}");
    }

    #[test]
    fn detail_check_multiligne_est_eclate_sans_etre_mange() {
        let mut app = app();
        app.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        app.apply(Event::CheckFinished {
            step: Step::Build,
            name: "pre_command".to_string(),
            ok: false,
            detail: "pre_command failed: make -C cat -n all\nmake: *** No rule to make target 'all'.  Stop."
                .to_string(),
        });
        va_a(&mut app, Selection::Step(Step::Build));
        let (s, _) = texte(&app, 90, 12);
        // Les deux lignes du détail sont visibles, non concaténées.
        assert!(
            s.contains("pre_command failed: make -C cat -n all"),
            "première ligne mangée\n{s}"
        );
        assert!(
            s.contains("No rule to make target"),
            "suite du détail mangée\n{s}"
        );
        assert!(
            !s.contains("allmake"),
            "lignes concaténées sans séparateur\n{s}"
        );
    }

    #[test]
    fn detail_scroll_back_remonte_dans_les_logs() {
        let mut app = app();
        app.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        for i in 0..30 {
            app.apply(Event::LogLine {
                step: Step::Build,
                line: format!("l{i:02}"),
            });
        }
        va_a(&mut app, Selection::Step(Step::Build));
        app.scroll_up();
        let (s, _) = texte(&app, 60, 12);
        assert!(!s.contains("l29"), "encore en bas malgré scroll_back\n{s}");
        assert!(s.contains("l19"), "recul de 10 lignes absent\n{s}");
    }

    #[test]
    fn detail_scrolle_gele_la_vue_tant_que_scroll_back_est_ouvert() {
        let mut app = app();
        app.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        for i in 0..30 {
            app.apply(Event::LogLine {
                step: Step::Build,
                line: format!("l{i:02}"),
            });
        }
        va_a(&mut app, Selection::Step(Step::Build));
        app.scroll_up();
        let (s1, _) = texte(&app, 60, 12);
        assert!(s1.contains("l19"), "recul initial absent\n{s1}");
        // Le run continue : de nouveaux logs arrivent pendant que
        // l'utilisateur lit — la vue ne doit PAS dériver.
        for i in 30..40 {
            app.apply(Event::LogLine {
                step: Step::Build,
                line: format!("l{i:02}"),
            });
        }
        let (s2, _) = texte(&app, 60, 12);
        assert_eq!(s1, s2, "la vue doit être gelée tant que scroll_back > 0");
        // Revenu en bas : le suivi reprend, les derniers logs sont là.
        app.scroll_down();
        let (s3, _) = texte(&app, 60, 12);
        assert!(s3.contains("l39"), "le suivi ne reprend pas à 0\n{s3}");
    }

    #[test]
    fn detail_norme_montre_le_compteur_des_fautes_cappees() {
        let mut app = app();
        for i in 0..crate::tui::dashboard::MAX_NORME + 4 {
            app.apply(Event::NormeFault(NormeFault {
                file: "src/a.c".into(),
                line: i as u32,
                col: 1,
                severity: Severity::Minor,
                rule: "C-G1".to_string(),
                message: "mauvais en-tête".to_string(),
            }));
        }
        va_a(&mut app, Selection::Step(Step::Norme));
        let (s, _) = texte(&app, 60, 12);
        assert!(
            s.contains("et 4 autres"),
            "compteur de reliquat absent\n{s}"
        );
    }

    #[test]
    fn detail_test_failed_montre_le_diff_colore() {
        let mut app = app();
        app.apply(Event::TestFinished {
            group: "unit".to_string(),
            name: "b".to_string(),
            result: TestVerdict::Failed {
                diff: "-x\n+y".to_string(),
                expected: "x".to_string(),
                got: "y".to_string(),
            },
        });
        va_a(&mut app, Selection::Test { group: 0, index: 0 });
        let (s, backend) = texte(&app, 60, 12);
        assert!(s.contains("-x"), "ligne - du diff absente\n{s}");
        assert!(s.contains("+y"), "ligne + du diff absente\n{s}");
        let buf = backend.buffer();
        // Couleur de la première cellule portant ce symbole (« - » et
        // « + » n'apparaissent que dans les lignes de diff).
        let couleur_de = |symbole: &str| {
            (0..buf.area.height)
                .find_map(|y| {
                    (0..buf.area.width)
                        .find(|&x| buf[(x, y)].symbol() == symbole)
                        .map(|x| buf[(x, y)].fg)
                })
                .unwrap_or(Color::Reset)
        };
        assert_eq!(couleur_de("-"), Color::Red, "ligne - non rouge");
        assert_eq!(couleur_de("+"), Color::Green, "ligne + non verte");
    }

    #[test]
    fn detail_test_crash_montre_le_signal() {
        let mut app = app();
        app.apply(Event::TestFinished {
            group: "unit".to_string(),
            name: "segv".to_string(),
            result: TestVerdict::Crashed(11),
        });
        va_a(&mut app, Selection::Test { group: 0, index: 0 });
        let (s, _) = texte(&app, 60, 12);
        assert!(s.contains("SIGSEGV"), "signal absent\n{s}");
    }

    #[test]
    fn detail_faute_montre_position_severite_message_regle() {
        let mut app = app();
        app.apply(Event::NormeFault(NormeFault {
            file: "src/a.c".into(),
            line: 3,
            col: 1,
            severity: Severity::Major,
            rule: "C-G1".to_string(),
            message: "mauvais en-tête".to_string(),
        }));
        va_a(&mut app, Selection::Fault(0));
        let (s, _) = texte(&app, 60, 12);
        for attendu in ["src/a.c", ":3:1", "major", "mauvais en-tête", "C-G1"] {
            assert!(s.contains(attendu), "{attendu} absent\n{s}");
        }
    }
}
