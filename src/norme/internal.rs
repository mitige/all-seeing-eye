//! Moteur de norme interne (fallback quand epiclang est absent).
//!
//! Implémente en Rust les règles mécaniques du coding style Epitech
//! v7.1, avec les messages exacts du plugin banana (calibrés sur
//! `epiclang -fsyntax-only`, clang 21). Ce n'est PAS un parseur C :
//! un lexer minimaliste suit commentaires, chaînes, caractères,
//! directives préprocesseur et profondeur d'accolades — suffisant pour
//! localiser fonctions, globales et les fautes ci-dessous, sans
//! prétendre analyser tout le C.
//!
//! Écarts connus et assumés vis-à-vis de banana (qui voit l'AST clang)
//! : déclarations reconnues par une liste de types usuels, paramètres
//! sur une seule ligne de prototype, fonctions détectées par
//! « identificateur + parenthèses + `{` à profondeur 0 ». Les messages,
//! sévérités et positions calibrés sont en revanche identiques.

use crate::norme::{NormeFault, Severity};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

/// Nature d'un fichier analysé par le moteur interne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Source `.c`.
    C,
    /// En-tête `.h` (C-H2 s'y ajoute).
    Header,
    /// `Makefile` / `*.mk` : seul l'en-tête Epitech (C-G1) y est vérifié.
    Makefile,
}

/// Classe un chemin de rendu : `.c` → [`SourceKind::C`], `.h` →
/// [`SourceKind::Header`], `Makefile`/`*.mk` → [`SourceKind::Makefile`],
/// autre → `None`.
pub fn kind_of(path: &Path) -> Option<SourceKind> {
    if path.file_name().is_some_and(|n| n == "Makefile") {
        return Some(SourceKind::Makefile);
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("c") => Some(SourceKind::C),
        Some("h") => Some(SourceKind::Header),
        Some("mk") => Some(SourceKind::Makefile),
        _ => None,
    }
}

/// Ordinal anglais suffixé : 1st, 2nd, 3rd, 4th… 11th, 12th, 13th,
/// 21st, 22nd, 23rd… (les dizaines 11-13 font exception).
pub(crate) fn ordinal(n: u32) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// En-tête Epitech standard d'un fichier C (calibré sur banana : la
/// ligne « ** EPITECH PROJECT, <année> » puis une ligne de nom de
/// projet puis « ** File description: » sont exigées en tête).
static C_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\A/\*\n\*\* EPITECH PROJECT, \d{4}\n\*\* .+\n\*\* File description:")
        .expect("regex header C invalide")
});

/// Variante Makefile : lignes préfixées `##`. Le plan mentionnait
/// `### EPITECH` — la forme réelle est `## EPITECH` ; on accepte les
/// deux (`#{2,3}`) par tolérance.
static MAKEFILE_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A##\n#{2,3} EPITECH PROJECT, \d{4}\n##+ .+\n##+ File description:")
        .expect("regex header Makefile invalide")
});

/// Garde contre la double inclusion (`#ifndef` ou `#pragma once`).
static INCLUDE_GUARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*#\s*(ifndef\b|pragma\s+once\b)").expect("regex garde"));

/// Résultat du lexage minimaliste d'un fichier.
struct Scan {
    /// Lignes brutes, sans le `\n` final mais avec le `\r` éventuel.
    raw_lines: Vec<String>,
    /// Mêmes lignes avec commentaires, chaînes, caractères et lignes
    /// préprocesseur blankés (espaces), positions en chars préservées.
    code_lines: Vec<String>,
    /// Plage (début, fin) de chaque commentaire, en (ligne, col)
    /// 1-based — le début pour C-F8, la couverture de lignes pour les
    /// lignes « vides » de C-L6 (un commentaire n'est pas du vide).
    comments: Vec<((u32, u32), (u32, u32))>,
}

/// États du lexer caractère par caractère.
#[derive(Clone, Copy, PartialEq)]
enum Lex {
    Code,
    LineComment,
    BlockComment,
    Str,
    Chr,
}

/// Lexe `content` : blankage des commentaires (`//`, `/* */`), des
/// littéraux chaîne/caractère (contenu blanké, quotes conservées) et
/// des lignes préprocesseur (entièrement blankées — un `#define` ne
/// doit pas parasiter le suivi d'accolades). Le `\n` n'est jamais
/// blanké ; le `\r` est blanké dans le code (il reste en brut pour
/// C-G6/C-G7).
fn scan(content: &str) -> Scan {
    // Découpage en lignes : la pièce vide finale d'un fichier terminé
    // par '\n' n'est pas une ligne.
    let mut raw_lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    if content.ends_with('\n') {
        raw_lines.pop();
    }
    let mut code_lines: Vec<Vec<char>> = Vec::with_capacity(raw_lines.len());
    let mut comments = Vec::new();
    let mut state = Lex::Code;
    let mut comment_start = (0u32, 0u32); // début du commentaire ouvert
    for (li, raw) in raw_lines.iter().enumerate() {
        let chars: Vec<char> = raw.chars().collect();
        let mut out = chars.clone();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            match state {
                Lex::Code => match (c, next) {
                    ('/', Some('/')) => {
                        comment_start = (li as u32 + 1, i as u32 + 1);
                        out[i] = ' ';
                        out[i + 1] = ' ';
                        i += 2;
                        state = Lex::LineComment;
                        continue;
                    }
                    ('/', Some('*')) => {
                        comment_start = (li as u32 + 1, i as u32 + 1);
                        out[i] = ' ';
                        out[i + 1] = ' ';
                        i += 2;
                        state = Lex::BlockComment;
                        continue;
                    }
                    ('"', _) => state = Lex::Str,
                    ('\'', _) => state = Lex::Chr,
                    ('\r', _) => out[i] = ' ',
                    _ => {}
                },
                Lex::LineComment => out[i] = ' ',
                Lex::BlockComment => {
                    out[i] = ' ';
                    if c == '*' && next == Some('/') {
                        out[i + 1] = ' ';
                        comments.push((comment_start, (li as u32 + 1, i as u32 + 2)));
                        i += 2;
                        state = Lex::Code;
                        continue;
                    }
                }
                Lex::Str => {
                    if c == '\\' && next.is_some() {
                        out[i] = ' ';
                        out[i + 1] = ' ';
                        i += 2;
                        continue;
                    }
                    if c == '"' {
                        state = Lex::Code;
                    } else {
                        out[i] = ' '; // contenu blanké, quotes conservées
                    }
                }
                Lex::Chr => {
                    if c == '\\' && next.is_some() {
                        out[i] = ' ';
                        out[i + 1] = ' ';
                        i += 2;
                        continue;
                    }
                    if c == '\'' {
                        state = Lex::Code;
                    } else {
                        out[i] = ' ';
                    }
                }
            }
            i += 1;
        }
        // Un commentaire de ligne se termine avec la ligne.
        if state == Lex::LineComment {
            comments.push((comment_start, (li as u32 + 1, chars.len() as u32)));
        }
        // Une chaîne ou un caractère ne peut pas couvrir deux lignes :
        // fin de ligne = fin d'état (le C invalide ne doit pas faire
        // dérailler les lignes suivantes).
        if matches!(state, Lex::Str | Lex::Chr | Lex::LineComment) {
            state = Lex::Code;
        }
        code_lines.push(out);
    }
    // Commentaire bloc jamais refermé (C invalide) : il court jusqu'à
    // la fin du fichier.
    if state == Lex::BlockComment {
        let last_l = raw_lines.len() as u32;
        let last_c = raw_lines.last().map_or(1, |l| l.chars().count() as u32);
        comments.push((comment_start, (last_l, last_c)));
    }
    // Préprocesseur : ligne dont le premier non-blanc brut est '#'.
    for (raw, out) in raw_lines.iter().zip(code_lines.iter_mut()) {
        if raw.trim_start().starts_with('#') {
            for ch in out.iter_mut() {
                *ch = ' ';
            }
        }
    }
    Scan {
        raw_lines,
        code_lines: code_lines
            .into_iter()
            .map(|l| l.into_iter().collect())
            .collect(),
        comments,
    }
}

/// Construit une faute positionnée.
fn fault(
    file: &Path,
    line: u32,
    col: u32,
    severity: Severity,
    rule: &str,
    message: String,
) -> NormeFault {
    NormeFault {
        file: file.to_path_buf(),
        line,
        col,
        severity,
        rule: rule.to_string(),
        message,
    }
}

/// C-G1 : en-tête Epitech standard en tout début de fichier.
fn check_header(file: &Path, content: &str, kind: SourceKind, out: &mut Vec<NormeFault>) {
    let re = match kind {
        SourceKind::Makefile => &MAKEFILE_HEADER_RE,
        _ => &C_HEADER_RE,
    };
    if !re.is_match(content) {
        out.push(fault(
            file,
            1,
            1,
            Severity::Minor,
            "C-G1",
            "file not starting with standard Epitech header".to_string(),
        ));
    }
}

/// Règles par ligne : C-G7 (espace/tab/`\r` terminal — banana pointe
/// le dernier char de la ligne, calibré), C-G6 (`\r` terminal),
/// C-F3 (> 80 chars hors `\r\n`, message = nombre de chars, col 81).
fn check_lines(file: &Path, s: &Scan, out: &mut Vec<NormeFault>) {
    for (li, raw) in s.raw_lines.iter().enumerate() {
        let line = li as u32 + 1;
        let n_chars = raw.chars().count() as u32;
        if raw.ends_with([' ', '\t', '\r']) {
            out.push(fault(
                file,
                line,
                n_chars,
                Severity::Minor,
                "C-G7",
                "trailing space".to_string(),
            ));
        }
        if raw.ends_with('\r') {
            out.push(fault(
                file,
                line,
                n_chars,
                Severity::Minor,
                "C-G6",
                "\\r-style line ending".to_string(),
            ));
        }
        let visible = raw.strip_suffix('\r').unwrap_or(raw).chars().count() as u32;
        if visible > 80 {
            out.push(fault(
                file,
                line,
                81,
                Severity::Major,
                "C-F3",
                format!("{visible}-character line"),
            ));
        }
    }
}

/// C-A3 : le fichier doit finir par un `\n` (faute au dernier char,
/// col = longueur + 1 — calibré).
fn check_final_newline(file: &Path, content: &str, s: &Scan, out: &mut Vec<NormeFault>) {
    if content.is_empty() || content.ends_with('\n') {
        return;
    }
    let last = s.raw_lines.len() as u32;
    let col = s
        .raw_lines
        .last()
        .map_or(1, |l| l.chars().count() as u32 + 1);
    out.push(fault(
        file,
        last,
        col,
        Severity::Info,
        "C-A3",
        "file not ending with a newline".to_string(),
    ));
}

/// C-H2 (headers uniquement) : sans `#ifndef`/`#pragma once`, faute
/// à la première ligne de contenu (hors commentaires — calibré : la
/// ligne du premier token après l'en-tête).
fn check_include_guard(file: &Path, content: &str, s: &Scan, out: &mut Vec<NormeFault>) {
    if INCLUDE_GUARD_RE.is_match(content) {
        return;
    }
    let line = s
        .code_lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .map(|i| i as u32 + 1)
        .unwrap_or(1);
    out.push(fault(
        file,
        line,
        1,
        Severity::Major,
        "C-H2",
        "file not protected against double inclusion".to_string(),
    ));
}

/// Analyse un fichier et renvoie ses fautes de norme (moteur interne).
pub fn check_file(file: &Path, content: &str, kind: SourceKind) -> Vec<NormeFault> {
    let mut out = Vec::new();
    check_header(file, content, kind, &mut out);
    if kind == SourceKind::Makefile {
        return out; // banana ne vérifie que l'en-tête d'un Makefile
    }
    let s = scan(content);
    check_lines(file, &s, &mut out);
    let (fonctions, segments, chars, lines) = structure(file, &s, &mut out);
    check_functions(file, &s, &fonctions, &mut out);
    check_globals(file, &chars, &lines, &segments, &mut out);
    check_goto(file, &s, &mut out);
    if kind == SourceKind::Header {
        check_include_guard(file, content, &s, &mut out);
    }
    check_final_newline(file, content, &s, &mut out);
    out
}

// ============================ structure ============================

/// Nature d'une accolade pour les règles de placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BraceKind {
    /// Corps de fonction.
    Function,
    /// Bloc de contrôle (if/for/while/do/else/switch) ou bloc anonyme.
    Block,
    /// Initialisation (`= {`, `({`, `, {`, `[ {`, `{ {`) : exempte des
    /// règles C-L1/C-L4 (calibré : `int a[3] = {1, 2, 3};` est propre).
    Aggregate,
    /// Corps de struct/union/enum : idem (calibré : `} point_t;` est
    /// la forme attendue d'un typedef struct).
    StructBody,
}

/// Fonction détectée par le scan structurel.
#[derive(Debug)]
struct FunctionInfo {
    name: String,
    /// (ligne, col) du nom.
    name_pos: (u32, u32),
    /// Ligne du début de la déclaration (pour C-O3 non-static : col 1).
    decl_line: u32,
    is_static: bool,
    /// (ligne, col) du `{` du corps.
    open: (u32, u32),
    /// (ligne, col) du `}` du corps.
    close: (u32, u32),
    /// (ligne, col) du premier token de chaque paramètre.
    params: Vec<(u32, u32)>,
}

/// Comment un segment de portée fichier se termine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegEnd {
    Semi,
    Open(BraceKind),
    Close,
}

/// Comment un segment de portée fichier commence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegStart {
    FileStart,
    AfterSemi,
    AfterOpen,
    AfterClose,
}

/// Segment de texte à profondeur 0 (entre deux frontières), candidat
/// à l'analyse C-G4.
#[derive(Debug)]
struct Segment {
    start: usize,
    end: usize,
    started: SegStart,
    ended: SegEnd,
}

/// Mots-clés de contrôle suivis de parenthèses : `kw (...) {` est un
/// bloc, pas une fonction.
const CONTROL_KW: &[&str] = &["if", "for", "while", "switch"];

/// Vrai si `c` est un caractère d'identificateur C.
fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Scan structurel du code blanké : accolades (fonctions, blocs,
/// agrégats, structs), C-L1 et C-L4 émis en ordre de scan, segments
/// de portée fichier collectés pour C-G4. Renvoie les fonctions
/// détectées (pour C-F2/F4/F5/F8/L6/O3).
///
/// Le scan travaille en INDICES DE CHARS (jamais d'octets) : les cols
/// banana sont des positions en caractères.
fn structure(
    file: &Path,
    s: &Scan,
    out: &mut Vec<NormeFault>,
) -> (Vec<FunctionInfo>, Vec<Segment>, Vec<char>, LineIndex) {
    let chars: Vec<char> = s.code_lines.join("\n").chars().collect();
    let lines = LineIndex::new(&chars);
    let mut st = StructScan {
        file,
        chars: &chars,
        lines: &lines,
        braces: Vec::new(),
        parens: Vec::new(),
        functions: Vec::new(),
        segments: Vec::new(),
        seg_start: 0,
        seg_started: SegStart::FileStart,
    };
    st.run(out);
    (st.functions, st.segments, chars, lines)
}

/// Index ligne ↔ offset (chars) sur le code aplati.
struct LineIndex {
    /// Offset (char) du premier caractère de chaque ligne.
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(chars: &[char]) -> Self {
        let mut starts = vec![0];
        for (i, &c) in chars.iter().enumerate() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        LineIndex { starts }
    }

    /// (ligne, col) 1-based d'un offset char.
    fn locate(&self, off: usize) -> (u32, u32) {
        let li = match self.starts.binary_search(&off) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        (li as u32 + 1, (off - self.starts[li]) as u32 + 1)
    }
}

/// État du scan structurel.
struct StructScan<'a> {
    file: &'a Path,
    chars: &'a [char],
    lines: &'a LineIndex,
    /// Pile des accolades ouvertes (profondeur = len).
    braces: Vec<(BraceKind, Option<usize>, Option<String>)>,
    /// Pile des parenthèses ouvertes : `true` si précédée de `for`.
    parens: Vec<bool>,
    functions: Vec<FunctionInfo>,
    segments: Vec<Segment>,
    /// Début (char offset) du segment de portée fichier courant.
    seg_start: usize,
    seg_started: SegStart,
}

impl StructScan<'_> {
    /// Parcours caractère par caractère.
    fn run(&mut self, out: &mut Vec<NormeFault>) {
        let mut ci = 0;
        while ci < self.chars.len() {
            match self.chars[ci] {
                '(' => {
                    let is_for = self.ident_before(ci).is_some_and(|(w, _)| w == "for");
                    self.parens.push(is_for);
                }
                ')' => {
                    self.parens.pop();
                }
                '{' => self.on_open_brace(ci, out),
                '}' => self.on_close_brace(ci, out),
                ';' => self.on_semi(ci, out),
                _ => {}
            }
            ci += 1;
        }
    }

    /// Précédent caractère non-blanc avant `ci`.
    fn prev_sig(&self, ci: usize) -> Option<usize> {
        (0..ci).rev().find(|&j| !self.chars[j].is_whitespace())
    }

    /// Identificateur se terminant juste avant `ci` (blancs ignorés).
    fn ident_before(&self, ci: usize) -> Option<(String, usize)> {
        let mut end = ci;
        while end > 0 && self.chars[end - 1].is_whitespace() {
            end -= 1;
        }
        let mut start = end;
        while start > 0 && is_ident(self.chars[start - 1]) {
            start -= 1;
        }
        (start < end).then(|| (self.chars[start..end].iter().collect(), start))
    }

    /// Parenthèse ouvrante appariée à la fermante en `close_ci`.
    fn paren_match_back(&self, close_ci: usize) -> Option<usize> {
        let mut depth = 0;
        for j in (0..=close_ci).rev() {
            match self.chars[j] {
                ')' => depth += 1,
                '(' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Premier caractère non-blanc après `ci` sur la même ligne.
    fn next_on_line(&self, ci: usize) -> Option<usize> {
        let mut j = ci + 1;
        while j < self.chars.len() && self.chars[j] != '\n' {
            if !self.chars[j].is_whitespace() {
                return Some(j);
            }
            j += 1;
        }
        None
    }

    /// Vrai si du code précède `ci` sur sa ligne.
    fn code_before_on_line(&self, ci: usize) -> bool {
        let (l, _) = self.lines.locate(ci);
        let start = self.lines.starts[l as usize - 1];
        (start..ci).any(|j| !self.chars[j].is_whitespace())
    }

    /// Le token en `ci` commence-t-il le mot `kw` (frontière incluse) ?
    fn word_at(&self, ci: usize, kw: &str) -> bool {
        let kw: Vec<char> = kw.chars().collect();
        if ci + kw.len() > self.chars.len() {
            return false;
        }
        if self.chars[ci..ci + kw.len()] != kw[..] {
            return false;
        }
        ci + kw.len() >= self.chars.len() || !is_ident(self.chars[ci + kw.len()])
    }

    /// Émet C-L1 « multiple statements » au token suivant `ci`, si
    /// présent sur la ligne et non exempté (`else`/`while` — calibré :
    /// `} else {` et `} while (...);` sont licites).
    fn l1_after(&self, ci: usize, exempt_words: bool, out: &mut Vec<NormeFault>) {
        let Some(t) = self.next_on_line(ci) else {
            return;
        };
        if exempt_words && (self.word_at(t, "else") || self.word_at(t, "while")) {
            return;
        }
        let (l, c) = self.lines.locate(t);
        out.push(fault(
            self.file,
            l,
            c,
            Severity::Major,
            "C-L1",
            "multiple statements on the same line".to_string(),
        ));
    }

    /// `{` : classification, C-L4 d'ouverture, C-L1, frontière de
    /// segment à profondeur 0.
    fn on_open_brace(&mut self, ci: usize, out: &mut Vec<NormeFault>) {
        let (kind, fn_idx, kw) = self.classify_open(ci);
        let (l, c) = self.lines.locate(ci);
        let before = self.code_before_on_line(ci);
        let after = self.next_on_line(ci).is_some();
        match kind {
            BraceKind::Function => {
                // Calibré brace_same.c : K&R → les DEUX messages.
                if before {
                    out.push(fault(
                        self.file,
                        l,
                        c,
                        Severity::Minor,
                        "C-L4",
                        "function body opening brace on same line as prototype".to_string(),
                    ));
                }
                if before || after {
                    out.push(fault(
                        self.file,
                        l,
                        c,
                        Severity::Minor,
                        "C-L4",
                        "function body opening brace not alone on its line".to_string(),
                    ));
                }
                if after {
                    self.l1_after(ci, false, out);
                }
            }
            BraceKind::Block => {
                // Calibré ifbrace.c : `{` seul sous la condition.
                if let Some(kw) = &kw {
                    let kw_line = self.kw_line(ci).unwrap_or(l);
                    if !before && kw_line < l {
                        out.push(fault(
                            self.file,
                            l,
                            c,
                            Severity::Minor,
                            "C-L4",
                            format!("{kw} opening brace not on same line as condition"),
                        ));
                    }
                }
                if after {
                    self.l1_after(ci, false, out);
                }
            }
            BraceKind::Aggregate | BraceKind::StructBody => {}
        }
        if self.braces.is_empty() {
            self.close_segment(ci, SegEnd::Open(kind));
        }
        self.braces.push((kind, fn_idx, kw));
    }

    /// Ligne du mot-clé de contrôle auquel appartient le `{` en `ci`.
    fn kw_line(&self, brace_ci: usize) -> Option<u32> {
        let prev = self.prev_sig(brace_ci)?;
        if self.chars[prev] == ')' {
            let open = self.paren_match_back(prev)?;
            let (_, kw_off) = self.ident_before(open)?;
            return Some(self.lines.locate(kw_off).0);
        }
        let (_, kw_off) = self.ident_before(brace_ci)?;
        Some(self.lines.locate(kw_off).0)
    }

    /// Classe le `{` en `ci` ; pour une fonction, l'enregistre et
    /// renvoie son index. Voir le module pour les heuristiques.
    fn classify_open(&mut self, ci: usize) -> (BraceKind, Option<usize>, Option<String>) {
        let Some(prev) = self.prev_sig(ci) else {
            return (BraceKind::StructBody, None, None);
        };
        match self.chars[prev] {
            ')' => {
                let Some(open_paren) = self.paren_match_back(prev) else {
                    return (BraceKind::Block, None, None);
                };
                let Some((ident, ident_off)) = self.ident_before(open_paren) else {
                    return (BraceKind::Block, None, None);
                };
                if CONTROL_KW.contains(&ident.as_str()) {
                    return (BraceKind::Block, None, Some(ident));
                }
                let idx = self.register_function(ident, ident_off, open_paren, prev, ci);
                (BraceKind::Function, Some(idx), None)
            }
            '=' | '(' | ',' | '[' | '{' => (BraceKind::Aggregate, None, None),
            c if is_ident(c) => match self.ident_before(ci) {
                Some((w, _)) if w == "else" || w == "do" => (BraceKind::Block, None, Some(w)),
                _ => (BraceKind::StructBody, None, None),
            },
            _ => (BraceKind::StructBody, None, None),
        }
    }

    /// Enregistre une fonction dont le `{` est en `brace_ci` : nom,
    /// paramètres (positions du premier token), staticité, ligne de
    /// déclaration.
    fn register_function(
        &mut self,
        name: String,
        name_off: usize,
        open_paren: usize,
        close_paren: usize,
        brace_ci: usize,
    ) -> usize {
        let params = split_params(self.chars, open_paren + 1, close_paren)
            .into_iter()
            .filter_map(|(a, b)| {
                (a..b)
                    .find(|&j| !self.chars[j].is_whitespace())
                    .map(|j| self.lines.locate(j))
            })
            .collect::<Vec<_>>();
        // Les paramètres « void » seul ou vide ne comptent pas.
        let params = if params.len() == 1 {
            let text: String = self.chars[open_paren + 1..close_paren]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if text == "void" {
                Vec::new()
            } else {
                params
            }
        } else {
            params
        };
        // Début de la déclaration : après la frontière `;`/`{`/`}` la
        // plus proche en amont du nom (portée fichier).
        let mut decl = name_off;
        while decl > 0 && !matches!(self.chars[decl - 1], ';' | '{' | '}') {
            decl -= 1;
        }
        while decl < name_off && self.chars[decl].is_whitespace() {
            decl += 1;
        }
        let is_static = self.chars[decl..name_off]
            .iter()
            .collect::<String>()
            .split(|c: char| !is_ident(c))
            .any(|w| w == "static");
        self.functions.push(FunctionInfo {
            name,
            name_pos: self.lines.locate(name_off),
            decl_line: self.lines.locate(decl).0,
            is_static,
            open: self.lines.locate(brace_ci),
            close: (0, 0), // posé à la fermeture
            params,
        });
        self.functions.len() - 1
    }

    /// `}` : C-L4 de fermeture, C-L1, frontière de segment.
    fn on_close_brace(&mut self, ci: usize, out: &mut Vec<NormeFault>) {
        let Some((kind, fn_idx, kw)) = self.braces.pop() else {
            return; // '}' orpheline : code invalide, on ignore
        };
        let (l, c) = self.lines.locate(ci);
        let before = self.code_before_on_line(ci);
        let after = self.next_on_line(ci);
        let exempt = after.is_some_and(|t| self.word_at(t, "else") || self.word_at(t, "while"));
        match kind {
            BraceKind::Function => {
                if before || after.is_some() {
                    out.push(fault(
                        self.file,
                        l,
                        c,
                        Severity::Minor,
                        "C-L4",
                        "function body closing brace not alone on its line".to_string(),
                    ));
                }
            }
            BraceKind::Block => {
                if let (Some(kw), false) = (&kw, exempt) {
                    if before || after.is_some() {
                        out.push(fault(
                            self.file,
                            l,
                            c,
                            Severity::Minor,
                            "C-L4",
                            format!("{kw} closing brace not alone on its line"),
                        ));
                    }
                }
            }
            BraceKind::Aggregate | BraceKind::StructBody => {}
        }
        if !matches!(kind, BraceKind::Aggregate | BraceKind::StructBody) && !exempt {
            self.l1_after(ci, false, out);
        }
        if let Some(idx) = fn_idx {
            self.functions[idx].close = (l, c);
        }
        if self.braces.is_empty() {
            self.close_segment(ci, SegEnd::Close);
        }
    }

    /// `;` : C-L1 si du code suit sur la ligne — UNIQUEMENT à
    /// profondeur ≥ 1 (calibré : à portée fichier, banana ne dit rien)
    /// et hors parenthèses de `for`.
    fn on_semi(&mut self, ci: usize, out: &mut Vec<NormeFault>) {
        if !self.braces.is_empty() && !self.parens.last().copied().unwrap_or(false) {
            self.l1_after(ci, false, out);
        }
        if self.braces.is_empty() {
            self.close_segment(ci, SegEnd::Semi);
        }
    }

    /// Clôt le segment de portée fichier `[seg_start, ci)` et ouvre le
    /// suivant après `ci`.
    fn close_segment(&mut self, ci: usize, ended: SegEnd) {
        self.segments.push(Segment {
            start: self.seg_start,
            end: ci,
            started: self.seg_started,
            ended,
        });
        self.seg_start = ci + 1;
        self.seg_started = match ended {
            SegEnd::Semi => SegStart::AfterSemi,
            SegEnd::Open(_) => SegStart::AfterOpen,
            SegEnd::Close => SegStart::AfterClose,
        };
    }
}

/// Découpe la plage de paramètres `a..b` sur les virgules de niveau 0.
fn split_params(chars: &[char], a: usize, b: usize) -> Vec<(usize, usize)> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = a;
    for (i, &c) in chars[a..b].iter().enumerate() {
        let j = a + i;
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push((start, j));
                start = j + 1;
            }
            _ => {}
        }
    }
    parts.push((start, b));
    parts
}

/// Règles par fonction : C-F2 (nom < 3), C-F5 (> 4 paramètres), C-F4
/// (> 20 lignes de corps, lignes vides comprises — calibré), C-L6
/// (ligne vide manquante après les déclarations), C-F8 (commentaire
/// dans le corps), C-O3 (> 5 non-statiques / > 10 au total).
fn check_functions(file: &Path, s: &Scan, fonctions: &[FunctionInfo], out: &mut Vec<NormeFault>) {
    for (i, f) in fonctions.iter().enumerate() {
        if f.name.chars().count() < 3 {
            out.push(fault(
                file,
                f.name_pos.0,
                f.name_pos.1,
                Severity::Minor,
                "C-F2",
                "function name too short".to_string(),
            ));
        }
        for (p, (pl, pc)) in f.params.iter().enumerate().skip(4) {
            out.push(fault(
                file,
                *pl,
                *pc,
                Severity::Major,
                "C-F5",
                format!("{} parameter in function", ordinal(p as u32 + 1)),
            ));
        }
        check_body_length(file, s, f, out);
        check_decl_section(file, s, f, out);
        // C-O3 — calibré : une faute par fonction au-delà de chaque
        // limite ; message combiné quand les deux sont dépassées.
        let total = i as u32 + 1;
        let ns = fonctions[..=i].iter().filter(|g| !g.is_static).count() as u32;
        let over_ns = ns > 5 && !f.is_static;
        let over_total = total > 10;
        let msg = match (over_ns, over_total) {
            (true, true) => format!(
                "{} non-static and {} function in the file",
                ordinal(ns),
                ordinal(total)
            ),
            (true, false) => format!("{} non-static function in the file", ordinal(ns)),
            (false, true) => format!("{} function in the file", ordinal(total)),
            (false, false) => String::new(),
        };
        if !msg.is_empty() {
            // Calibré : col 1 pour une non-statique, col du nom pour
            // une statique.
            let (l, c) = if f.is_static {
                f.name_pos
            } else {
                (f.decl_line, 1)
            };
            out.push(fault(file, l, c, Severity::Major, "C-O3", msg));
        }
    }
    // C-F8 : commentaire strictement entre les accolades d'une fonction.
    for &((cl, cc), _) in &s.comments {
        if fonctions
            .iter()
            .any(|f| (cl, cc) > f.open && (cl, cc) < f.close)
        {
            out.push(fault(
                file,
                cl,
                cc,
                Severity::Minor,
                "C-F8",
                "comment inside function".to_string(),
            ));
        }
    }
}

/// C-F4 : une faute Major par ligne de corps au-delà de la 20e, col 1.
/// L'ordinal est le décalage PHYSIQUE depuis la ligne du `{` (les
/// lignes vides comptent — calibré fn_blank.c). La ligne du `}` ne
/// compte que si du code la précède sur la ligne.
fn check_body_length(file: &Path, s: &Scan, f: &FunctionInfo, out: &mut Vec<NormeFault>) {
    let (open_l, _) = f.open;
    let (close_l, close_c) = f.close;
    for l in open_l + 1..=close_l {
        if l == close_l {
            let code = &s.code_lines[l as usize - 1];
            let avant: String = code.chars().take(close_c as usize - 1).collect();
            if avant.trim().is_empty() {
                break; // `}` seul sur sa ligne : ne compte pas
            }
        }
        let n = l - open_l;
        if n > 20 {
            out.push(fault(
                file,
                l,
                1,
                Severity::Major,
                "C-F4",
                format!("{} line in the function", ordinal(n)),
            ));
        }
    }
}

/// Ligne de déclaration de variable (heuristique : types usuels du
/// pool Epitech, `t_*`, `*_t` ; les appels `f(...)` ne matchent pas).
static DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:(?:static|const|volatile|register|unsigned|signed|long|short)\s+)*(?:int|char|float|double|void|bool|size_t|ssize_t|off_t|pid_t|struct\s+\w+|enum\s+\w+|union\s+\w+|\w+_t|t_\w+)\s+[\*\s]*[A-Za-z_]\w*(?:\s*\[[^\];]*\])*\s*(?:=[^;]*)?;\s*$",
    )
    .expect("regex déclaration invalide")
});

/// Famille C-L6 (toutes Minor, col 1 — calibré) :
/// - « leading blank line in function body » : ligne vide en tête de
///   corps (blankstart.c) ;
/// - « missing blank line after variable declaration section » :
///   déclarations suivies directement de code (decl.c) ;
/// - « blank line inside non-variable-declaration section » : ligne
///   vide entre instructions (blankmid.c) ;
/// - « trailing blank line in function body » : ligne vide avant le
///   `}` final (blankend.c).
///
/// La ligne vide séparant la section de déclarations du code est la
/// SEULE licite.
fn check_decl_section(file: &Path, s: &Scan, f: &FunctionInfo, out: &mut Vec<NormeFault>) {
    let (open_l, _) = f.open;
    let (close_l, close_c) = f.close;
    // Code d'une ligne de corps ; sur la ligne du `}`, seul le code
    // AVANT l'accolade compte (un `}` seul n'est pas une ligne vide).
    let code_of = |l: u32| -> String {
        let line = &s.code_lines[l as usize - 1];
        if l == close_l {
            line.chars().take(close_c as usize - 1).collect()
        } else {
            line.clone()
        }
    };
    // « Vide » au sens banana : pas de code ET pas de commentaire (une
    // ligne de commentaire est fautive via C-F8, pas via C-L6).
    let ligne_vide = |l: u32| -> bool {
        code_of(l).trim().is_empty()
            && !s.comments.iter().any(|((a, _), (b, _))| *a <= l && l <= *b)
    };
    let mut l = open_l + 1;
    // 1. Lignes vides en tête de corps.
    while l < close_l && ligne_vide(l) {
        out.push(fault(
            file,
            l,
            1,
            Severity::Minor,
            "C-L6",
            "leading blank line in function body".to_string(),
        ));
        l += 1;
    }
    // 2. Section de déclarations.
    let mut vus = false;
    while l < close_l && DECL_RE.is_match(&code_of(l)) {
        vus = true;
        l += 1;
    }
    // 3. Après les déclarations : une ligne vide (le séparateur), ou
    // du code directement → « missing blank line ».
    if vus && l < close_l {
        if ligne_vide(l) {
            l += 1; // le séparateur, unique ligne vide licite
        } else {
            out.push(fault(
                file,
                l,
                1,
                Severity::Minor,
                "C-L6",
                "missing blank line after variable declaration section".to_string(),
            ));
        }
    }
    // 4. Section d'instructions : toute ligne vide est fautive —
    // « trailing » si plus aucune instruction ne suit, « inside »
    // sinon.
    while l < close_l {
        if ligne_vide(l) {
            let suite_code =
                (l + 1..close_l).any(|k| !ligne_vide(k)) || !code_of(close_l).trim().is_empty();
            let msg = if suite_code {
                "blank line inside non-variable-declaration section"
            } else {
                "trailing blank line in function body"
            };
            out.push(fault(file, l, 1, Severity::Minor, "C-L6", msg.to_string()));
        }
        l += 1;
    }
}

/// C-G4 : variables globales non-const. Segments de portée fichier
/// entre `;`/`{`/`}` ; prototypes et typedefs exclus ; `const` et
/// `extern` exemptés (calibré) ; une faute par déclarateur, col = nom.
fn check_globals(
    file: &Path,
    chars: &[char],
    lines: &LineIndex,
    segments: &[Segment],
    out: &mut Vec<NormeFault>,
) {
    let mut typedef_open = false;
    for seg in segments {
        // À l'intérieur d'une accolade (membres de struct, corps) :
        // pas des globales.
        if seg.started == SegStart::AfterOpen {
            continue;
        }
        let text: String = chars[seg.start..seg.end].iter().collect();
        let first = text
            .split(|c: char| !is_ident(c))
            .find(|w| !w.is_empty())
            .unwrap_or("");
        // Une chaîne typedef masque tous ses segments jusqu'au `;`.
        if typedef_open {
            if seg.ended == SegEnd::Semi {
                typedef_open = false;
            }
            continue;
        }
        if first == "typedef" {
            typedef_open = seg.ended != SegEnd::Semi;
            continue;
        }
        // En-tête de fonction ou de struct/enum/union : pas une
        // variable. Un `{` d'initialiseur (Aggregate) reste analysable
        // (« int a[] = » avant le `{`).
        if matches!(
            seg.ended,
            SegEnd::Open(BraceKind::Function | BraceKind::StructBody | BraceKind::Block)
        ) {
            continue;
        }
        if first == "extern" {
            continue; // calibré : une déclaration extern n'est pas posée
        }
        let mots: Vec<&str> = text
            .split(|c: char| !is_ident(c))
            .filter(|w| !w.is_empty())
            .collect();
        if mots.contains(&"const") {
            continue;
        }
        // Prototype ou pointeur de fonction : `(` avant tout `=`.
        let paren = text.find('(');
        let egal = text.find('=');
        if paren.is_some_and(|p| egal.is_none_or(|e| p < e)) {
            continue;
        }
        // Un nom par déclarateur (virgules de niveau 0).
        for (a, b) in split_params(chars, seg.start, seg.end) {
            if let Some(name_off) = declarator_name(chars, a, b) {
                let (l, c) = lines.locate(name_off);
                out.push(fault(
                    file,
                    l,
                    c,
                    Severity::Major,
                    "C-G4",
                    "global variable".to_string(),
                ));
            }
        }
    }
}

/// Offset du nom de variable dans un déclarateur `a..b` : dernier
/// identificateur avant le premier `=` (crochets `[...]` ignorés).
/// `int g_counter` → `g_counter` ; `char *g_str` → `g_str` ;
/// `int a[10]` → `a`. `None` si aucun ident au-delà du type.
fn declarator_name(chars: &[char], a: usize, b: usize) -> Option<usize> {
    let mut end = b;
    let mut depth = 0;
    for (i, &c) in chars[a..b].iter().enumerate() {
        let j = a + i;
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '=' if depth == 0 => {
                end = j;
                break;
            }
            _ => {}
        }
    }
    // Idents hors crochets (ex. la taille d'un tableau n'est pas un nom).
    let mut last: Option<(usize, usize)> = None; // (start, end)
    let mut j = a;
    let mut bracket = 0;
    while j < end {
        match chars[j] {
            '[' => bracket += 1,
            ']' => bracket -= 1,
            c if is_ident(c) && bracket == 0 => {
                let start = j;
                while j < end && is_ident(chars[j]) {
                    j += 1;
                }
                last = Some((start, j));
                continue;
            }
            _ => {}
        }
        j += 1;
    }
    // Au moins deux idents (type + nom) OU un seul ident si le
    // déclarateur n'a pas de type visible (segment après `}` : « g_p »).
    last.map(|(start, _)| start)
}

/// C-C3 : `goto` hors commentaires/chaînes (calibré : col du mot).
fn check_goto(file: &Path, s: &Scan, out: &mut Vec<NormeFault>) {
    for (li, code) in s.code_lines.iter().enumerate() {
        let mut start = 0;
        let chars: Vec<char> = code.chars().collect();
        while let Some(pos) = find_word(&chars, start, "goto") {
            out.push(fault(
                file,
                li as u32 + 1,
                pos as u32 + 1,
                Severity::Major,
                "C-C3",
                "use of goto statement".to_string(),
            ));
            start = pos + 4;
        }
    }
}

/// Première occurrence du mot `kw` (frontières non-ident) dans
/// `chars` à partir de `start`, en indices de chars.
fn find_word(chars: &[char], start: usize, kw: &str) -> Option<usize> {
    let kw: Vec<char> = kw.chars().collect();
    (start..chars.len()).find(|&i| {
        chars[i..].starts_with(&kw)
            && (i == 0 || !is_ident(chars[i - 1]))
            && (i + kw.len() >= chars.len() || !is_ident(chars[i + kw.len()]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// En-tête Epitech valide (6 lignes) + ligne vide (ligne 7).
    const HEADER: &str =
        "/*\n** EPITECH PROJECT, 2026\n** proj\n** File description:\n** x\n*/\n\n";

    fn fautes(src: &str) -> Vec<NormeFault> {
        check_file(Path::new("t.c"), src, SourceKind::C)
    }

    fn regle<'a>(f: &'a [NormeFault], rule: &str) -> Vec<&'a NormeFault> {
        f.iter().filter(|x| x.rule == rule).collect()
    }

    // ------------------------- ordinal -------------------------

    #[test]
    fn ordinal_suffixes() {
        let attendus = [
            (1, "1st"),
            (2, "2nd"),
            (3, "3rd"),
            (4, "4th"),
            (11, "11th"),
            (12, "12th"),
            (13, "13th"),
            (21, "21st"),
            (22, "22nd"),
            (23, "23rd"),
            (24, "24th"),
            (111, "111th"),
            (112, "112th"),
            (113, "113th"),
            (121, "121st"),
        ];
        for (n, att) in attendus {
            assert_eq!(ordinal(n), att, "ordinal({n})");
        }
    }

    // ------------------------- C-G1 (en-tête) -------------------------

    #[test]
    fn g1_sans_header_minor_a_1_1() {
        let f = fautes("int my_add(int a, int b)\n{\n    return a + b;\n}\n");
        let g1 = regle(&f, "C-G1");
        assert_eq!(g1.len(), 1, "une seule C-G1 attendue : {f:?}");
        assert_eq!(g1[0].severity, Severity::Minor);
        assert_eq!((g1[0].line, g1[0].col), (1, 1));
        assert_eq!(
            g1[0].message,
            "file not starting with standard Epitech header"
        );
        assert_eq!(g1[0].file, PathBuf::from("t.c"));
    }

    #[test]
    fn g1_header_valide_propre() {
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n    return a + b;\n}}\n");
        assert!(regle(&fautes(&src), "C-G1").is_empty());
    }

    #[test]
    fn g1_makefile_variant_diese() {
        let bon = "##\n## EPITECH PROJECT, 2026\n## proj\n## File description:\n## x\n##\n\nall:\n";
        let f = check_file(Path::new("Makefile"), bon, SourceKind::Makefile);
        assert!(
            regle(&f, "C-G1").is_empty(),
            "header Makefile valide : {f:?}"
        );

        let mauvais = "all:\n\ttrue\n";
        let f = check_file(Path::new("Makefile"), mauvais, SourceKind::Makefile);
        assert_eq!(regle(&f, "C-G1").len(), 1, "header manquant : {f:?}");
    }

    // ------------------------- C-G7 (trailing space) -------------------------

    #[test]
    fn g7_trailing_space_col_dernier_char() {
        // Calibré : banana pointe le DERNIER caractère de la ligne.
        let ligne = "    return a + b;   "; // 17 + 3 espaces = 20 chars
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n{ligne}\n}}\n");
        let f = fautes(&src);
        let g7 = regle(&f, "C-G7");
        assert_eq!(g7.len(), 1, "{f:?}");
        assert_eq!(g7[0].severity, Severity::Minor);
        assert_eq!(g7[0].line, 10);
        assert_eq!(g7[0].col as usize, ligne.chars().count());
        assert_eq!(g7[0].message, "trailing space");
    }

    #[test]
    fn g7_tab_compte_pour_un_char() {
        let ligne = "    return a;\t "; // tab = 1 char
        let src = format!("{HEADER}int f(int a)\n{{\n{ligne}\n}}\n");
        let f = fautes(&src);
        let g7 = regle(&f, "C-G7");
        assert_eq!(g7.len(), 1);
        assert_eq!(g7[0].col as usize, ligne.chars().count());
    }

    // ------------------------- C-G6 (\r) -------------------------

    #[test]
    fn g6_crlf_signale_chaque_ligne() {
        let src = "/*\r\n** EPITECH PROJECT, 2026\r\n";
        let f = fautes(src);
        let g6 = regle(&f, "C-G6");
        assert_eq!(g6.len(), 2, "{f:?}");
        assert_eq!(g6[0].severity, Severity::Minor);
        assert_eq!(g6[0].message, "\\r-style line ending");
        assert_eq!((g6[0].line, g6[0].col), (1, 3)); // "/*\r" : \r en col 3
    }

    // ------------------------- C-F3 (ligne > 80) -------------------------

    #[test]
    fn f3_ligne_de_80_ok_81_ko() {
        // 80 chars visibles : propre.
        let l80 = format!("    return {};", "a".repeat(68)); // 11 + 68 + 1 = 80
        assert_eq!(l80.chars().count(), 80);
        let src = format!("{HEADER}int my_f(int a)\n{{\n{l80}\n    return a;\n}}\n");
        assert!(regle(&fautes(&src), "C-F3").is_empty());

        // 81 chars : faute, col 81, message = nombre de chars.
        let l81 = format!("    return {};", "a".repeat(69)); // 81
        assert_eq!(l81.chars().count(), 81);
        let src = format!("{HEADER}int my_f(int a)\n{{\n{l81}\n    return a;\n}}\n");
        let f = fautes(&src);
        let f3 = regle(&f, "C-F3");
        assert_eq!(f3.len(), 1);
        assert_eq!(f3[0].severity, Severity::Major);
        assert_eq!(f3[0].message, "81-character line");
        assert_eq!((f3[0].line, f3[0].col), (10, 81));
    }

    // ------------------------- C-A3 (newline finale) -------------------------

    #[test]
    fn a3_fichier_sans_newline_finale() {
        let src = format!("{HEADER}int my_f(void)\n{{\n    return 0;\n}}"); // pas de \n final
        let f = fautes(&src);
        let a3 = regle(&f, "C-A3");
        assert_eq!(a3.len(), 1, "{f:?}");
        assert_eq!(a3[0].severity, Severity::Info);
        assert_eq!(a3[0].message, "file not ending with a newline");
        assert_eq!((a3[0].line, a3[0].col), (11, 2)); // "}" : 1 char, col = len+1
    }

    #[test]
    fn a3_newline_finale_presente() {
        let src = format!("{HEADER}int my_f(void)\n{{\n    return 0;\n}}\n");
        assert!(regle(&fautes(&src), "C-A3").is_empty());
    }

    // ------------------------- C-H2 (include guard) -------------------------

    #[test]
    fn h2_header_sans_garde() {
        let src = format!("{HEADER}int my_add(int a, int b);\n");
        let f = check_file(Path::new("t.h"), &src, SourceKind::Header);
        let h2 = regle(&f, "C-H2");
        assert_eq!(h2.len(), 1, "{f:?}");
        assert_eq!(h2[0].severity, Severity::Major);
        assert_eq!(h2[0].message, "file not protected against double inclusion");
        assert_eq!((h2[0].line, h2[0].col), (8, 1)); // 1re ligne de contenu
    }

    #[test]
    fn h2_header_avec_garde_propre() {
        let src = format!(
            "{HEADER}#ifndef T_H_\n    #define T_H_\n\nint my_add(int a, int b);\n\n#endif\n"
        );
        let f = check_file(Path::new("t.h"), &src, SourceKind::Header);
        assert!(regle(&f, "C-H2").is_empty(), "{f:?}");
    }

    #[test]
    fn h2_ne_concerne_pas_les_c() {
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n    return a + b;\n}}\n");
        assert!(regle(&fautes(&src), "C-H2").is_empty());
    }

    // ------------------------- anti faux positifs -------------------------

    #[test]
    fn fichier_canonique_propre() {
        let src = format!(
            "{HEADER}int my_add(int a, int b)\n{{\n    int c;\n\n    c = a + b;\n    return c;\n}}\n"
        );
        let f = fautes(&src);
        assert!(f.is_empty(), "fichier propre flaggé : {f:?}");
    }

    #[test]
    fn controle_de_flux_propre() {
        // if/else : `} else {` est la forme correcte (calibré).
        let src = format!(
            "{HEADER}int my_abs(int a)\n{{\n    if (a < 0) {{\n        return -a;\n    }} else {{\n        return a;\n    }}\n}}\n"
        );
        assert_eq!(fautes(&src), vec![], "if/else");
        // for : les `;` de l'en-tête ne sont pas des C-L1 (calibré).
        let src = format!(
            "{HEADER}int my_acc(int n)\n{{\n    int s = 0;\n\n    for (int i = 0; i < n; i++) {{\n        s += i;\n    }}\n    return s;\n}}\n"
        );
        assert_eq!(fautes(&src), vec![], "for");
        // do-while : `} while (...);` est la forme correcte (calibré).
        let src = format!(
            "{HEADER}int my_loop(int a)\n{{\n    do {{\n        a--;\n    }} while (a > 0);\n    return a;\n}}\n"
        );
        assert_eq!(fautes(&src), vec![], "do-while");
        // initialisation de tableau : `= {1, 2, 3}` n'est pas un bloc.
        let src = format!(
            "{HEADER}int my_sum(void)\n{{\n    int arr[3] = {{1, 2, 3}};\n\n    return arr[0] + arr[1] + arr[2];\n}}\n"
        );
        assert_eq!(fautes(&src), vec![], "initializer");
    }

    // ------------------------- C-F2 (nom < 3) -------------------------

    #[test]
    fn f2_nom_trop_court() {
        let src = format!("{HEADER}int ab(int a)\n{{\n    return a;\n}}\n");
        let f = fautes(&src);
        let f2 = regle(&f, "C-F2");
        assert_eq!(f2.len(), 1, "{f:?}");
        assert_eq!(f2[0].severity, Severity::Minor);
        assert_eq!(f2[0].message, "function name too short");
        assert_eq!((f2[0].line, f2[0].col), (8, 5)); // position du nom
        let src = format!("{HEADER}int abc(int a)\n{{\n    return a;\n}}\n");
        assert!(regle(&fautes(&src), "C-F2").is_empty());
    }

    // ------------------------- C-F4 (> 20 lignes de corps) -------------------------

    /// Fonction my_acc : prototype ligne 8, `{` ligne 9, `n_incr`
    /// lignes « a += i » à partir de la ligne 10, puis return et `}`.
    fn fn_avec_corps(n_incr: usize) -> String {
        let mut s = format!("{HEADER}int my_acc(int a)\n{{\n");
        for i in 0..n_incr {
            s.push_str(&format!("    a += {i};\n"));
        }
        s.push_str("    return a;\n}\n");
        s
    }

    #[test]
    fn f4_corps_de_20_lignes_ok_21_ko() {
        // 19 + return = 20 lignes de corps : propre (calibré).
        assert!(regle(&fautes(&fn_avec_corps(19)), "C-F4").is_empty());
        // 20 + return = 21 lignes : la 21e (le return, ligne 30) est KO.
        let f = fautes(&fn_avec_corps(20));
        let f4 = regle(&f, "C-F4");
        assert_eq!(f4.len(), 1, "{f:?}");
        assert_eq!(f4[0].severity, Severity::Major);
        assert_eq!(f4[0].message, "21st line in the function");
        assert_eq!((f4[0].line, f4[0].col), (30, 1));
    }

    #[test]
    fn f4_une_faute_par_ligne_excedentaire() {
        // 22 + return = 23 lignes de corps (calibré fn23.c).
        let f = fautes(&fn_avec_corps(22));
        let f4 = regle(&f, "C-F4");
        assert_eq!(f4.len(), 3, "{f:?}");
        assert_eq!(f4[0].message, "21st line in the function");
        assert_eq!((f4[0].line, f4[0].col), (30, 1));
        assert_eq!(f4[1].message, "22nd line in the function");
        assert_eq!((f4[1].line, f4[1].col), (31, 1));
        assert_eq!(f4[2].message, "23rd line in the function");
        assert_eq!((f4[2].line, f4[2].col), (32, 1));
    }

    #[test]
    fn f4_les_lignes_vides_comptent() {
        // Calibré : 9 code + 3 vides + 8 code + return = 21 lignes
        // physiques → faute sur la 21e (le return, ligne 30).
        let mut s = format!("{HEADER}int my_acc(int a)\n{{\n");
        for i in 0..9 {
            s.push_str(&format!("    a += {i};\n"));
        }
        s.push_str("\n\n\n");
        for i in 9..17 {
            s.push_str(&format!("    a += {i};\n"));
        }
        s.push_str("    return a;\n}\n");
        let f = fautes(&s);
        let f4 = regle(&f, "C-F4");
        assert_eq!(f4.len(), 1, "{f:?}");
        assert_eq!(f4[0].message, "21st line in the function");
        assert_eq!(f4[0].line, 30);
    }

    // ------------------------- C-F5 (> 4 paramètres) -------------------------

    #[test]
    fn f5_cinquieme_parametre() {
        // Calibré : col = 1er token du paramètre excédentaire.
        let src = format!(
            "{HEADER}int my_sum(int a, int b, int c, int d, int e)\n{{\n    return a;\n}}\n"
        );
        let f = fautes(&src);
        let f5 = regle(&f, "C-F5");
        assert_eq!(f5.len(), 1, "{f:?}");
        assert_eq!(f5[0].severity, Severity::Major);
        assert_eq!(f5[0].message, "5th parameter in function");
        assert_eq!((f5[0].line, f5[0].col), (8, 40));
    }

    #[test]
    fn f5_quatre_ok_six_deux_fautes() {
        let src =
            format!("{HEADER}int my_sum(int a, int b, int c, int d)\n{{\n    return a;\n}}\n");
        assert!(regle(&fautes(&src), "C-F5").is_empty());
        let src = format!(
            "{HEADER}int my_sum(int a, int b, int c, int d, int e, int f)\n{{\n    return a;\n}}\n"
        );
        let f = fautes(&src);
        let f5 = regle(&f, "C-F5");
        assert_eq!(f5.len(), 2, "{f:?}");
        assert_eq!(f5[0].message, "5th parameter in function");
        assert_eq!((f5[0].line, f5[0].col), (8, 40));
        assert_eq!(f5[1].message, "6th parameter in function");
        assert_eq!((f5[1].line, f5[1].col), (8, 47));
    }

    #[test]
    fn f5_void_ne_compte_pas() {
        let src = format!("{HEADER}int my_f(void)\n{{\n    return 0;\n}}\n");
        assert!(regle(&fautes(&src), "C-F5").is_empty());
    }

    // ------------------------- C-F8 (commentaire dans fonction) -------------------------

    #[test]
    fn f8_commentaires_dans_fonction() {
        // Calibré comment.c.
        let src = format!(
            "{HEADER}int my_add(int a, int b)\n{{\n    // add them\n    return a /* inline */ + b;\n}}\n"
        );
        let f = fautes(&src);
        let f8 = regle(&f, "C-F8");
        assert_eq!(f8.len(), 2, "{f:?}");
        assert_eq!(f8[0].severity, Severity::Minor);
        assert_eq!(f8[0].message, "comment inside function");
        assert_eq!((f8[0].line, f8[0].col), (10, 5));
        assert_eq!((f8[1].line, f8[1].col), (11, 14));
    }

    #[test]
    fn f8_commentaires_hors_fonction_ok() {
        // L'en-tête Epitech et un commentaire hors fonction sont licites.
        let src = format!("{HEADER}// avant\nint my_f(void)\n{{\n    return 0;\n}}\n// apres\n");
        assert!(regle(&fautes(&src), "C-F8").is_empty());
    }

    // ------------------------- C-O3 (trop de fonctions) -------------------------

    /// `n` fonctions de 4 lignes à la suite : fn i démarre ligne 8+4i.
    fn fonctions(n: usize, statiques: bool) -> String {
        let mut s = HEADER.to_string();
        for i in 0..n {
            let kw = if statiques { "static " } else { "" };
            s.push_str(&format!(
                "{kw}int my_fn{i:02}(int a)\n{{\n    return a + {i};\n}}\n"
            ));
        }
        s
    }

    #[test]
    fn o3_sixieme_fonction_non_statique() {
        // Calibré : une faute par fonction non-statique au-delà de la 5e.
        let f = fautes(&fonctions(6, false));
        let o3 = regle(&f, "C-O3");
        assert_eq!(o3.len(), 1, "{f:?}");
        assert_eq!(o3[0].severity, Severity::Major);
        assert_eq!(o3[0].message, "6th non-static function in the file");
        assert_eq!((o3[0].line, o3[0].col), (28, 1));
    }

    #[test]
    fn o3_onzieme_combine_les_deux_limites() {
        // Calibré many.c : fns 6..10 → « Nth non-static » ; la 11e
        // (non-statique) dépasse les deux limites → message combiné.
        let f = fautes(&fonctions(11, false));
        let o3 = regle(&f, "C-O3");
        assert_eq!(o3.len(), 6, "{f:?}");
        assert_eq!(o3[4].message, "10th non-static function in the file");
        assert_eq!(
            o3[5].message,
            "11th non-static and 11th function in the file"
        );
        assert_eq!((o3[5].line, o3[5].col), (48, 1));
    }

    #[test]
    fn o3_onzieme_fonction_statique() {
        // Calibré statics.c : seule la limite totale (> 10) joue ;
        // la col est celle du NOM pour une fonction statique.
        let f = fautes(&fonctions(11, true));
        let o3 = regle(&f, "C-O3");
        assert_eq!(o3.len(), 1, "{f:?}");
        assert_eq!(o3[0].message, "11th function in the file");
        assert_eq!((o3[0].line, o3[0].col), (48, 12));
    }

    #[test]
    fn o3_cinq_non_statiques_ok() {
        assert!(regle(&fautes(&fonctions(5, false)), "C-O3").is_empty());
    }

    // ------------------------- C-G4 (variable globale) -------------------------

    #[test]
    fn g4_globales_non_const() {
        // Calibré global.c + global2.c : const et extern exemptes,
        // static flaggée, col = position du nom.
        let src = format!(
            "{HEADER}int g_counter = 0;\nconst int g_max = 3;\nstatic int g_priv;\nextern int g_ext;\nint g_arr[10];\n\nint my_add(int a, int b)\n{{\n    return a + b;\n}}\n"
        );
        let f = fautes(&src);
        let g4 = regle(&f, "C-G4");
        assert_eq!(g4.len(), 3, "{f:?}");
        assert_eq!(g4[0].severity, Severity::Major);
        assert_eq!(g4[0].message, "global variable");
        assert_eq!((g4[0].line, g4[0].col), (8, 5)); // g_counter
        assert_eq!((g4[1].line, g4[1].col), (10, 12)); // g_priv
        assert_eq!((g4[2].line, g4[2].col), (12, 5)); // g_arr
    }

    #[test]
    fn g4_deux_globales_une_par_instruction() {
        // Calibré filescope.c : pas de C-L1 à portée fichier, une
        // C-G4 par déclaration.
        let src =
            format!("{HEADER}int g_a = 5; int g_b = 6;\n\nint my_f(void)\n{{\n    return 0;\n}}\n");
        let f = fautes(&src);
        let g4 = regle(&f, "C-G4");
        assert_eq!(g4.len(), 2, "{f:?}");
        assert_eq!((g4[0].line, g4[0].col), (8, 5));
        assert_eq!((g4[1].line, g4[1].col), (8, 18));
        assert!(
            regle(&f, "C-L1").is_empty(),
            "pas de C-L1 file scope : {f:?}"
        );
    }

    #[test]
    fn g4_typedef_struct_exempts_mais_variable_instance_flaggee() {
        let src = format!(
            "{HEADER}typedef struct point {{\n    int x;\n    int y;\n}} point_t;\n\nint my_f(void)\n{{\n    return 0;\n}}\n"
        );
        assert!(regle(&fautes(&src), "C-G4").is_empty());
        let src = format!(
            "{HEADER}struct point {{\n    int x;\n    int y;\n}} g_p;\n\nint my_f(void)\n{{\n    return 0;\n}}\n"
        );
        let f = fautes(&src);
        let g4 = regle(&f, "C-G4");
        assert_eq!(g4.len(), 1, "{f:?}");
        assert_eq!((g4[0].line, g4[0].col), (11, 3)); // « g_p » après « } »
    }

    #[test]
    fn g4_prototype_non_flagge() {
        let src = format!("{HEADER}int my_f(int a);\n\nint my_f(int a)\n{{\n    return a;\n}}\n");
        assert!(regle(&fautes(&src), "C-G4").is_empty());
    }

    // ------------------------- C-L1 (plusieurs instructions) -------------------------

    #[test]
    fn l1_deux_instructions_meme_ligne() {
        // Calibré multi.c : col = début de l'instruction suivante.
        let src = format!(
            "{HEADER}int my_add(int a, int b)\n{{\n    a += 1; b += 1;\n    return a + b;\n}}\n"
        );
        let f = fautes(&src);
        let l1 = regle(&f, "C-L1");
        assert_eq!(l1.len(), 1, "{f:?}");
        assert_eq!(l1[0].severity, Severity::Major);
        assert_eq!(l1[0].message, "multiple statements on the same line");
        assert_eq!((l1[0].line, l1[0].col), (10, 13));
    }

    #[test]
    fn l1_bloc_inline_deux_fautes() {
        // Calibré inline_brace.c : `{` suivi de code, puis `;` suivi
        // de `}` — deux C-L1.
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n    if (a > 0) {{ a += 1; }}\n    return a + b;\n}}\n");
        let f = fautes(&src);
        let l1 = regle(&f, "C-L1");
        assert_eq!(l1.len(), 2, "{f:?}");
        assert_eq!((l1[0].line, l1[0].col), (10, 18));
        assert_eq!((l1[1].line, l1[1].col), (10, 26));
    }

    #[test]
    fn l1_accolade_fermante_puis_instruction() {
        // Calibré ifret.c : `} return` → C-L1 au token qui suit.
        let src = format!("{HEADER}int my_abs(int a)\n{{\n    if (a < 0) {{\n        a = -a;\n    }} return a;\n}}\n");
        let f = fautes(&src);
        let l1 = regle(&f, "C-L1");
        assert_eq!(l1.len(), 1, "{f:?}");
        assert_eq!((l1[0].line, l1[0].col), (12, 7));
    }

    // ------------------------- C-L4 (placement d'accolades) -------------------------

    #[test]
    fn l4_ouvrante_fonction_sur_ligne_du_prototype() {
        // Calibré brace_same.c : les DEUX messages, à la col du `{`.
        let src = format!("{HEADER}int my_add(int a, int b) {{\n    return a + b;\n}}\n");
        let f = fautes(&src);
        let l4 = regle(&f, "C-L4");
        assert_eq!(l4.len(), 2, "{f:?}");
        assert_eq!(l4[0].severity, Severity::Minor);
        assert_eq!(
            l4[0].message,
            "function body opening brace on same line as prototype"
        );
        assert_eq!((l4[0].line, l4[0].col), (8, 26));
        assert_eq!(
            l4[1].message,
            "function body opening brace not alone on its line"
        );
        assert_eq!((l4[1].line, l4[1].col), (8, 26));
    }

    #[test]
    fn l4_fermante_fonction_pas_seule() {
        // Calibré brace_close.c.
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n    return a + b; }}\n");
        let f = fautes(&src);
        let l4 = regle(&f, "C-L4");
        assert_eq!(l4.len(), 1, "{f:?}");
        assert_eq!(
            l4[0].message,
            "function body closing brace not alone on its line"
        );
        assert_eq!((l4[0].line, l4[0].col), (10, 19));
    }

    #[test]
    fn l4_ouvrante_if_sous_la_condition() {
        // Calibré ifbrace.c.
        let src = format!("{HEADER}int my_abs(int a)\n{{\n    if (a < 0)\n    {{\n        a = -a;\n    }}\n    return a;\n}}\n");
        let f = fautes(&src);
        let l4 = regle(&f, "C-L4");
        assert_eq!(l4.len(), 1, "{f:?}");
        assert_eq!(
            l4[0].message,
            "if opening brace not on same line as condition"
        );
        assert_eq!((l4[0].line, l4[0].col), (11, 5));
    }

    #[test]
    fn l4_fermante_if_pas_seule() {
        // Calibré ifret.c.
        let src = format!("{HEADER}int my_abs(int a)\n{{\n    if (a < 0) {{\n        a = -a;\n    }} return a;\n}}\n");
        let f = fautes(&src);
        let l4 = regle(&f, "C-L4");
        assert_eq!(l4.len(), 1, "{f:?}");
        assert_eq!(l4[0].message, "if closing brace not alone on its line");
        assert_eq!((l4[0].line, l4[0].col), (12, 5));
    }

    #[test]
    fn l4_fermante_while_pas_seule() {
        // Calibré whileclose.c : le message porte le mot-clé du bloc.
        let src = format!("{HEADER}int my_dec(int a)\n{{\n    while (a > 0) {{\n        a--;\n    }} a += 1;\n    return a;\n}}\n");
        let f = fautes(&src);
        let l4 = regle(&f, "C-L4");
        assert_eq!(l4.len(), 1, "{f:?}");
        assert_eq!(l4[0].message, "while closing brace not alone on its line");
        assert_eq!((l4[0].line, l4[0].col), (12, 5));
    }

    // ------------------------- C-L6 (ligne vide après déclarations) -------------------------

    #[test]
    fn l6_ligne_vide_manquante() {
        // Calibré decl.c : faute à la ligne qui suit les déclarations.
        let src = format!("{HEADER}int my_add(int a, int b)\n{{\n    int c = a;\n    int d = b;\n    return c + d;\n}}\n");
        let f = fautes(&src);
        let l6 = regle(&f, "C-L6");
        assert_eq!(l6.len(), 1, "{f:?}");
        assert_eq!(l6[0].severity, Severity::Minor);
        assert_eq!(
            l6[0].message,
            "missing blank line after variable declaration section"
        );
        assert_eq!((l6[0].line, l6[0].col), (12, 1));
    }

    #[test]
    fn l6_cas_licites() {
        // Déclarations + ligne vide + code : propre (decl_ok.c).
        let src = format!(
            "{HEADER}int my_add(int a, int b)\n{{\n    int c = a;\n\n    return c + b;\n}}\n"
        );
        assert!(regle(&fautes(&src), "C-L6").is_empty());
        // Déclarations seules en fin de corps : propre (declonly.c).
        let src = format!("{HEADER}void my_void(void)\n{{\n    int a;\n}}\n");
        assert!(regle(&fautes(&src), "C-L6").is_empty());
        // Pas de déclarations : pas de section, pas de faute.
        let src = format!("{HEADER}int my_f(int a)\n{{\n    a += 1;\n    return a;\n}}\n");
        assert!(regle(&fautes(&src), "C-L6").is_empty());
    }

    #[test]
    fn l6_ligne_vide_en_tete_de_corps() {
        // Calibré blankstart.c / blankstart2.c.
        let src = format!("{HEADER}int my_f(int a)\n{{\n\n    int c;\n\n    return a + c;\n}}\n");
        let f = fautes(&src);
        let l6 = regle(&f, "C-L6");
        assert_eq!(l6.len(), 1, "{f:?}");
        assert_eq!(l6[0].message, "leading blank line in function body");
        assert_eq!((l6[0].line, l6[0].col), (10, 1));
    }

    #[test]
    fn l6_ligne_vide_entre_instructions() {
        // Calibré blankmid.c : une faute par ligne vide intérieure.
        let src = format!(
            "{HEADER}int my_f(int a)\n{{\n    a += 1;\n\n    a += 2;\n\n    a += 3;\n    return a;\n}}\n"
        );
        let f = fautes(&src);
        let l6 = regle(&f, "C-L6");
        assert_eq!(l6.len(), 2, "{f:?}");
        assert_eq!(
            l6[0].message,
            "blank line inside non-variable-declaration section"
        );
        assert_eq!((l6[0].line, l6[0].col), (11, 1));
        assert_eq!((l6[1].line, l6[1].col), (13, 1));
    }

    #[test]
    fn l6_ligne_vide_avant_accolade_fermante() {
        // Calibré blankend.c.
        let src = format!(
            "{HEADER}int my_f(int a)\n{{\n    int c;\n\n    c = a + 1;\n    return c;\n\n}}\n"
        );
        let f = fautes(&src);
        let l6 = regle(&f, "C-L6");
        assert_eq!(l6.len(), 1, "{f:?}");
        assert_eq!(l6[0].message, "trailing blank line in function body");
        assert_eq!((l6[0].line, l6[0].col), (14, 1));
    }

    #[test]
    fn l6_ligne_de_commentaire_nest_pas_vide() {
        // Parité banana (comment.c / fn_comment.c) : une ligne de
        // commentaire — déjà fautive via C-F8 — ne compte PAS comme
        // ligne vide pour C-L6.
        let src = format!(
            "{HEADER}int my_f(int a)\n{{\n    // c1\n    a += 1;\n    /* c2 */\n    a += 2;\n    return a;\n}}\n"
        );
        let f = fautes(&src);
        assert!(
            regle(&f, "C-L6").is_empty(),
            "commentaires pris pour des lignes vides : {f:?}"
        );
        assert_eq!(regle(&f, "C-F8").len(), 2, "C-F8 attendues : {f:?}");
    }

    // ------------------------- C-C3 (goto) -------------------------

    #[test]
    fn c3_goto_flagge() {
        // Calibré gotofile.c.
        let src = format!(
            "{HEADER}int my_add(int a, int b)\n{{\n    goto end;\nend:\n    return a + b;\n}}\n"
        );
        let f = fautes(&src);
        let c3 = regle(&f, "C-C3");
        assert_eq!(c3.len(), 1, "{f:?}");
        assert_eq!(c3[0].severity, Severity::Major);
        assert_eq!(c3[0].message, "use of goto statement");
        assert_eq!((c3[0].line, c3[0].col), (10, 5));
    }

    #[test]
    fn c3_goto_dans_chaine_ou_commentaire_ignores() {
        let src = format!(
            "{HEADER}int my_f(void)\n{{\n    char *s = \"goto there\";\n\n    my_puts(s); // goto?\n    return 0;\n}}\n"
        );
        assert!(regle(&fautes(&src), "C-C3").is_empty());
        // Un identifiant contenant « goto » n'est pas un goto.
        let src = format!(
            "{HEADER}int my_f(void)\n{{\n    int my_goto = 0;\n\n    return my_goto;\n}}\n"
        );
        assert!(regle(&fautes(&src), "C-C3").is_empty());
    }
}
