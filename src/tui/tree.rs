//! Panneau gauche du dashboard : l'arbre étapes → feuilles (Task 11).
//!
//! 7 étapes racines avec icônes (`·` pending, spinner running, `✓` ok,
//! `✗` ko, `⤼` skipped) ; sous Norme ses fautes, sous Unit/Functional
//! leurs tests en feuilles (`✓` `✗` `💥` `⏱`) ; jauge `passed/total
//! (%)` par étape de tests (agrégée si plusieurs groupes rattachés).
//! La ligne sélectionnée est surlignée (REVERSED) pleine largeur et
//! l'arbre défile pour la garder visible. Le rendu lit [`App`] sans
//! jamais le muter.

use crate::engine::events::{Step, TestVerdict};
use crate::norme::Severity;
use crate::tui::dashboard::{groupe_etape, App, Selection, StepStatus};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Rend l'arbre dans `area`.
pub(crate) fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" étapes ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return; // terminal minuscule : dégradé, jamais panic
    }
    let rows = app.visible_rows();
    let lignes: Vec<Line> = rows.iter().map(|s| ligne(app, *s)).collect();
    // Défilement minimal : la sélection reste visible (collée en bas
    // si elle dépasse).
    let sel = rows.iter().position(|s| *s == app.selected());
    let h = inner.height as usize;
    let start = match sel {
        Some(i) if i >= h => i + 1 - h,
        _ => 0,
    };
    f.render_widget(
        Paragraph::new(lignes).scroll((start.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
    // Surlignage pleine largeur (bordures comprises) de la sélection.
    if let Some(i) = sel {
        if i >= start && i - start < h {
            let y = inner.y + (i - start) as u16;
            f.buffer_mut().set_style(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                Style::default().add_modifier(Modifier::REVERSED),
            );
        }
    }
}

/// Une ligne de l'arbre pour une [`Selection`].
fn ligne(app: &App, sel: Selection) -> Line<'static> {
    match sel {
        Selection::Step(step) => ligne_step(app, step),
        Selection::Fault(i) => ligne_fault(app, i),
        Selection::Test { group, index } => ligne_test(app, group, index),
    }
}

/// Icône + couleur du statut d'une étape.
fn icone_step(app: &App, status: StepStatus) -> (String, Color) {
    match status {
        StepStatus::Pending => ("·".to_string(), Color::DarkGray),
        StepStatus::Running => (app.spinner_char().to_string(), Color::Yellow),
        StepStatus::Ok => ("✓".to_string(), Color::Green),
        StepStatus::Ko => ("✗".to_string(), Color::Red),
        StepStatus::Skipped => ("⤼".to_string(), Color::DarkGray),
    }
}

/// Ligne d'une étape racine : icône, label, jauge si des groupes de
/// tests y sont rattachés (post-run : pourcentages finaux — mêmes
/// données).
fn ligne_step(app: &App, step: Step) -> Line<'static> {
    let st = app.step(step);
    let (icone, couleur) = icone_step(app, st.status);
    let mut spans = vec![
        Span::styled(format!("{icone} "), Style::default().fg(couleur)),
        Span::raw(st.label.clone()),
    ];
    if let Some((passed, total, pct)) = jauge(app, step) {
        spans.push(Span::styled(
            format!(" {passed}/{total} ({pct:.1}%)"),
            Style::default().fg(couleur),
        ));
    }
    Line::from(spans)
}

/// Ligne d'une faute de norme, colorée par sévérité.
fn ligne_fault(app: &App, i: usize) -> Line<'static> {
    let faute = &app.norme()[i];
    let couleur = match faute.severity {
        Severity::Fatal | Severity::Major => Color::Red,
        Severity::Minor => Color::Yellow,
        Severity::Info => Color::DarkGray,
    };
    Line::from(Span::styled(
        format!("    ⚠ {}:{}", faute.file.display(), faute.line),
        Style::default().fg(couleur),
    ))
}

/// Ligne d'un test en feuille : icône de verdict (spinner tant qu'il
/// tourne) + nom.
fn ligne_test(app: &App, group: usize, index: usize) -> Line<'static> {
    let t = &app.groups()[group].tests[index];
    let (icone, couleur) = match &t.verdict {
        None => (app.spinner_char().to_string(), Color::Yellow),
        Some(TestVerdict::Passed) => ("✓".to_string(), Color::Green),
        Some(TestVerdict::Failed { .. }) => ("✗".to_string(), Color::Red),
        Some(TestVerdict::Crashed(_)) => ("💥".to_string(), Color::Red),
        Some(TestVerdict::Timeout) => ("⏱".to_string(), Color::Yellow),
    };
    Line::from(vec![
        Span::styled(format!("    {icone} "), Style::default().fg(couleur)),
        Span::raw(t.name.clone()),
    ])
}

/// Jauge d'une étape de tests : (passed, total, % arrondi au dixième)
/// agrégés sur les groupes rattachés ; None si aucun test.
fn jauge(app: &App, step: Step) -> Option<(usize, usize, f64)> {
    let mut passed = 0;
    let mut total = 0;
    for g in app.groups() {
        if groupe_etape(&g.name) == step {
            passed += g.passed();
            total += g.total();
        }
    }
    if total == 0 {
        return None;
    }
    let pct = (1000.0 * passed as f64 / total as f64).round() / 10.0;
    Some((passed, total, pct))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::{Event, Step, TestVerdict};
    use crate::norme::{NormeFault, Severity};
    use crate::tui::dashboard::{App, Selection};
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;
    use ratatui::Terminal;
    use std::time::Instant;

    fn app_garnie() -> App {
        let mut app = App::new("mini".to_string(), Instant::now());
        let fault = |file: &str, line: u32, sev: Severity| NormeFault {
            file: file.into(),
            line,
            col: 1,
            severity: sev,
            rule: "C-G1".to_string(),
            message: "mauvais en-tête".to_string(),
        };
        app.apply(Event::NormeFault(fault("src/a.c", 3, Severity::Major)));
        app.apply(Event::NormeFault(fault("src/b.c", 7, Severity::Minor)));
        for (group, name, v) in [
            ("unit", "a", TestVerdict::Passed),
            (
                "unit",
                "b",
                TestVerdict::Failed {
                    diff: "-x\n+y".to_string(),
                    expected: "x".to_string(),
                    got: "y".to_string(),
                },
            ),
            ("functional", "c", TestVerdict::Passed),
        ] {
            app.apply(Event::TestFinished {
                group: group.to_string(),
                name: name.to_string(),
                result: v,
            });
        }
        app
    }

    /// Rend l'arbre dans un TestBackend et retourne le texte ligne par
    /// ligne (chaque cellule → son symbole).
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

    #[test]
    fn arbre_affiche_etapes_feuilles_et_jauges() {
        let app = app_garnie();
        let (s, _) = texte(&app, 60, 24);
        for label in [
            "Vérifications préliminaires",
            "Compilation",
            "Norme",
            "Symboles",
            "Tests unitaires",
            "Tests fonctionnels",
            "Verdict",
        ] {
            assert!(s.contains(label), "label manquant : {label}\n{s}");
        }
        // Feuilles : fautes de norme et tests avec icônes.
        assert!(s.contains("src/a.c"), "faute absente\n{s}");
        assert!(s.contains("✓"), "icône passed absente\n{s}");
        assert!(s.contains("✗"), "icône failed absente\n{s}");
        assert!(s.contains('a'), "test a absent\n{s}");
        // Jauge du groupe unit : 1 passed / 2 total → 50.0 %.
        assert!(s.contains("1/2"), "jauge compteurs absente\n{s}");
        assert!(s.contains("50.0"), "jauge pourcentage absente\n{s}");
    }

    #[test]
    fn ligne_selectionnee_est_mise_en_evidence() {
        let app = app_garnie();
        // Sélection initiale : Prelim (première ligne de l'arbre).
        assert_eq!(app.selected(), Selection::Step(Step::Prelim));
        let (_, backend) = texte(&app, 60, 24);
        let buf = backend.buffer();
        let y = (0..buf.area.height)
            .find(|&y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .contains("Vérifications préliminaires")
            })
            .expect("ligne Prelim affichée");
        assert!(
            buf[(0, y)].modifier.contains(Modifier::REVERSED),
            "ligne sélectionnée non surlignée"
        );
    }

    #[test]
    fn arbre_long_garde_la_selection_visible() {
        let mut app = App::new("mini".to_string(), Instant::now());
        for i in 0..30 {
            app.apply(Event::TestFinished {
                group: "unit".to_string(),
                name: format!("t{i:02}"),
                result: TestVerdict::Passed,
            });
        }
        // Sélection : le dernier test.
        while app.selected()
            != (Selection::Test {
                group: 0,
                index: 29,
            })
        {
            app.next();
        }
        let (s, _) = texte(&app, 60, 10);
        assert!(s.contains("t29"), "sélection hors écran\n{s}");
    }

    #[test]
    fn petit_terminal_ne_panique_pas() {
        let app = app_garnie();
        // 12×4 : bien sous le 80×24 conseillé — dégradé, jamais panic.
        let _ = texte(&app, 12, 4);
    }
}
