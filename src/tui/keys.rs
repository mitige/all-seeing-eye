//! Mapping touches → actions du dashboard (Task 11).
//!
//! Fonction pure : la boucle (`mod.rs`) applique l'[`Action`], la
//! traduction est testée hors terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Action déclenchée par une touche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Quit,
    Next,
    Prev,
    ScrollUp,
    ScrollDown,
    ToggleFailures,
    Export,
}

/// Traduit une touche en action ; None si non gérée. Seuls les
/// `Press`/`Repeat` comptent — crossterm émet aussi des `Release` sur
/// certains terminaux : sans ce garde-fou, chaque touche agirait deux
/// fois.
pub(crate) fn action_for(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let action = match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => Action::Quit,
        (_, KeyCode::Char('q')) => Action::Quit,
        (_, KeyCode::Char('f')) => Action::ToggleFailures,
        (_, KeyCode::Char('e')) => Action::Export,
        (_, KeyCode::Down | KeyCode::Char('j')) => Action::Next,
        (_, KeyCode::Up | KeyCode::Char('k')) => Action::Prev,
        (_, KeyCode::PageUp) => Action::ScrollUp,
        (_, KeyCode::PageDown) => Action::ScrollDown,
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn touche(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn touches_canoniques_mappees() {
        assert_eq!(action_for(touche(KeyCode::Char('q'))), Some(Action::Quit));
        assert_eq!(
            action_for(touche(KeyCode::Char('f'))),
            Some(Action::ToggleFailures)
        );
        assert_eq!(action_for(touche(KeyCode::Char('e'))), Some(Action::Export));
        assert_eq!(action_for(touche(KeyCode::Char('j'))), Some(Action::Next));
        assert_eq!(action_for(touche(KeyCode::Down)), Some(Action::Next));
        assert_eq!(action_for(touche(KeyCode::Char('k'))), Some(Action::Prev));
        assert_eq!(action_for(touche(KeyCode::Up)), Some(Action::Prev));
        assert_eq!(action_for(touche(KeyCode::PageUp)), Some(Action::ScrollUp));
        assert_eq!(
            action_for(touche(KeyCode::PageDown)),
            Some(Action::ScrollDown)
        );
    }

    #[test]
    fn ctrl_c_quitte_aussi() {
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn touches_inconnues_ignorees() {
        assert_eq!(action_for(touche(KeyCode::Char('x'))), None);
        assert_eq!(action_for(touche(KeyCode::Enter)), None);
        assert_eq!(action_for(touche(KeyCode::F(1))), None);
    }

    #[test]
    fn release_ne_declenche_jamais_deux_fois() {
        // crossterm émet Press/Repeat/Release sur certains terminaux :
        // seul Press compte, sinon chaque touche agirait deux fois.
        let mut rel = touche(KeyCode::Char('q'));
        rel.kind = KeyEventKind::Release;
        assert_eq!(action_for(rel), None);
        let mut rep = touche(KeyCode::Char('j'));
        rep.kind = KeyEventKind::Repeat;
        assert_eq!(action_for(rep), Some(Action::Next));
    }
}
