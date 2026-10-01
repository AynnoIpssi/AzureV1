use std::fmt;

use crate::compiler::rsh::models::token::{Position, Spanned, TokenRsH};

#[derive(Debug, Clone)]
pub enum AstNode {
    // -------------------- Elements --------------------
    Container {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Title {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Title1 {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Title2 {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Title3 {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Text {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Button {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    Image {
        class: String,
        id: String,
        src: String,
        children: Vec<AstNode>,
    },
    Video {
        class: String,
        id: String,
        src: String,
        children: Vec<AstNode>,
    },
    Textarea {
        class: String,
        id: String,
        children: Vec<AstNode>,
    },
    RawText(String),
    /// Composant ou champ (`<carte titre="...">`, `<checkbox#cgu/>`...) :
    /// resolu par l'interpreteur (voir `compiler::components`).
    Element {
        tag: String,
        class: String,
        id: String,
        attrs: Vec<(String, String)>,
        children: Vec<AstNode>,
    },

    // -------------------- Conditions/boucles --------------------
    // La condition est laissee brute (String) : son interpretation est geree ailleurs.
    If {
        condition: String,
        children: Vec<AstNode>,
    },
    ElseIf {
        condition: String,
        children: Vec<AstNode>,
    },
    Else {
        condition: String,
        children: Vec<AstNode>,
    },
    Match {
        condition: String,
        children: Vec<AstNode>,
    },
    /// Une branche de `Match` : `pattern` est le motif brut porte par
    /// `TokenRsH::Arm` (un litteral, plusieurs valeurs separees par `|`, ou
    /// `_` pour le catch-all - voir `condition::matches_pattern`). Un `Arm`
    /// rencontre HORS d'un `Match` (jamais consomme par lui, voir
    /// `interpreter::build_node`/`codegen::emit_node`) est tolere comme une
    /// anomalie de template, meme traitement qu'un `ElseIf`/`Else` orphelin.
    Arm {
        pattern: String,
        children: Vec<AstNode>,
    },
    While {
        condition: String,
        children: Vec<AstNode>,
    },
    For {
        condition: String,
        children: Vec<AstNode>,
    },
    ForEach {
        condition: String,
        children: Vec<AstNode>,
    },
}

/// Une erreur de parsing, rattachee a l'endroit du source ou elle a ete detectee.
#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: Position,
}

impl ParseError {
    fn new(kind: ParseErrorKind, pos: Position) -> Self {
        ParseError { kind, pos }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseErrorKind {
    /// Le flux de tokens s'arrete au milieu d'une construction (ex: `<` sans rien apres).
    UnexpectedEndOfInput,
    /// Apres un `<`, le token trouve n'est pas un element/une structure de controle connue.
    ExpectedElement(TokenRsH),
    /// Un `>` etait attendu pour fermer la balise.
    ExpectedCloseTag(Option<TokenRsH>),
    /// La balise ouverte n'a jamais ete refermee avant la fin du flux.
    UnclosedElement(TokenRsH),
    /// La balise fermante ne correspond pas a la balise ouverte (ex: `<container>...<!title>`).
    MismatchedClosingTag {
        expected: TokenRsH,
        found: TokenRsH,
    },
    /// Une balise fermante est presente sans balise ouvrante correspondante.
    UnexpectedClosingTag(TokenRsH),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {}, colonne {} : {}", self.pos.line, self.pos.column, self.kind)
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::UnexpectedEndOfInput => {
                write!(f, "fin de flux inattendue")
            }
            ParseErrorKind::ExpectedElement(found) => {
                write!(f, "un element etait attendu apres '<', trouve {found:?}")
            }
            ParseErrorKind::ExpectedCloseTag(found) => {
                write!(f, "'>' attendu, trouve {found:?}")
            }
            ParseErrorKind::UnclosedElement(token) => {
                write!(f, "element {token:?} jamais referme")
            }
            ParseErrorKind::MismatchedClosingTag { expected, found } => {
                write!(
                    f,
                    "balise fermante incorrecte : attendu quelque chose comme <!{expected:?}>, trouve {found:?}"
                )
            }
            ParseErrorKind::UnexpectedClosingTag(token) => {
                write!(f, "balise fermante {token:?} sans balise ouvrante correspondante")
            }
        }
    }
}

impl std::error::Error for ParseError {}

// L'erreur garde le token fautif (pour un message precis) : elle est grosse,
// mais rare et rendue une seule fois.
#[allow(clippy::result_large_err)]
pub fn parse(tokens: Vec<Spanned<TokenRsH>>) -> Result<Vec<AstNode>, ParseError> {
    let mut i = 0;
    let nodes = parse_nodes(&tokens, &mut i)?;

    // S'il reste un token, ce ne peut etre qu'un EndTagMarker sans element ouvert
    // correspondant (c'est la seule raison pour laquelle parse_nodes s'arrete tot).
    if let Some(spanned) = tokens.get(i) {
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedClosingTag(spanned.value.clone()),
            spanned.pos,
        ));
    }

    Ok(nodes)
}

// Parse des noeuds freres jusqu'a la fin du flux ou jusqu'a une balise
// fermante (EndTagMarker), qui est alors laissee pour l'appelant.
#[allow(clippy::result_large_err)]
fn parse_nodes(tokens: &[Spanned<TokenRsH>], i: &mut usize) -> Result<Vec<AstNode>, ParseError> {
    let mut nodes = Vec::new();

    while *i < tokens.len() {
        match &tokens[*i].value {
            TokenRsH::OpenTag => {
                let tag_pos = tokens[*i].pos;
                *i += 1;

                let elem_token = match tokens.get(*i) {
                    Some(spanned) => spanned.value.clone(),
                    None => return Err(ParseError::new(ParseErrorKind::UnexpectedEndOfInput, tag_pos)),
                };
                let elem_pos = tokens[*i].pos;

                if !is_element_token(&elem_token) {
                    return Err(ParseError::new(ParseErrorKind::ExpectedElement(elem_token), elem_pos));
                }
                *i += 1;

                match tokens.get(*i) {
                    Some(spanned) if spanned.value == TokenRsH::CloseTag => *i += 1,
                    Some(spanned) => {
                        return Err(ParseError::new(
                            ParseErrorKind::ExpectedCloseTag(Some(spanned.value.clone())),
                            spanned.pos,
                        ));
                    }
                    None => {
                        return Err(ParseError::new(ParseErrorKind::ExpectedCloseTag(None), elem_pos));
                    }
                }

                // `<divider/>` : ni enfants ni balise fermante.
                if let TokenRsH::Element(tag, class, id, attrs, true) = &elem_token {
                    nodes.push(AstNode::Element { tag: tag.clone(), class: class.clone(), id: id.clone(), attrs: attrs.clone(), children: Vec::new() });
                    continue;
                }

                let children = parse_nodes(tokens, i)?;

                // Balise fermante correspondante : <!element>
                match tokens.get(*i) {
                    Some(spanned) if spanned.value == TokenRsH::EndTagMarker => {
                        *i += 1;

                        let closing_token = match tokens.get(*i) {
                            Some(spanned) => spanned.value.clone(),
                            None => {
                                return Err(ParseError::new(
                                    ParseErrorKind::UnclosedElement(elem_token),
                                    elem_pos,
                                ));
                            }
                        };
                        let closing_pos = tokens[*i].pos;

                        let same_component = match (&closing_token, &elem_token) {
                            (TokenRsH::Element(a, ..), TokenRsH::Element(b, ..)) => a == b,
                            _ => true,
                        };
                        if std::mem::discriminant(&closing_token) != std::mem::discriminant(&elem_token) || !same_component {
                            return Err(ParseError::new(
                                ParseErrorKind::MismatchedClosingTag {
                                    expected: elem_token,
                                    found: closing_token,
                                },
                                closing_pos,
                            ));
                        }
                        *i += 1;

                        match tokens.get(*i) {
                            Some(spanned) if spanned.value == TokenRsH::CloseTag => *i += 1,
                            Some(spanned) => {
                                return Err(ParseError::new(
                                    ParseErrorKind::ExpectedCloseTag(Some(spanned.value.clone())),
                                    spanned.pos,
                                ));
                            }
                            None => {
                                return Err(ParseError::new(ParseErrorKind::ExpectedCloseTag(None), closing_pos));
                            }
                        }
                    }
                    _ => {
                        return Err(ParseError::new(
                            ParseErrorKind::UnclosedElement(elem_token),
                            elem_pos,
                        ));
                    }
                }

                // is_element_token a deja garanti que build_node reussira.
                let node = build_node(elem_token, children)
                    .expect("elem_token valide par is_element_token");
                nodes.push(node);
            }
            TokenRsH::EndTagMarker => {
                // Fermeture du noeud parent : on remonte sans consommer.
                break;
            }
            TokenRsH::RawText(text) => {
                nodes.push(AstNode::RawText(text.clone()));
                *i += 1;
            }
            _ => {
                *i += 1;
            }
        }
    }

    Ok(nodes)
}

// Seule source de verite sur les tokens valides apres un `<` : on delegue a
// build_node plutot que de dupliquer la liste des variantes ailleurs.
fn is_element_token(token: &TokenRsH) -> bool {
    build_node(token.clone(), Vec::new()).is_some()
}

// Convertit un token d'element (+ ses enfants deja parses) en noeud d'AST type.
fn build_node(token: TokenRsH, children: Vec<AstNode>) -> Option<AstNode> {
    match token {
        TokenRsH::Container(class, id) => Some(AstNode::Container { class, id, children }),
        TokenRsH::Title(class, id) => Some(AstNode::Title { class, id, children }),
        TokenRsH::Title1(class, id) => Some(AstNode::Title1 { class, id, children }),
        TokenRsH::Title2(class, id) => Some(AstNode::Title2 { class, id, children }),
        TokenRsH::Title3(class, id) => Some(AstNode::Title3 { class, id, children }),
        TokenRsH::Text(class, id) => Some(AstNode::Text { class, id, children }),
        TokenRsH::Button(class, id) => Some(AstNode::Button { class, id, children }),
        TokenRsH::Image(class, id, src) => Some(AstNode::Image { class, id, src, children }),
        TokenRsH::Video(class, id, src) => Some(AstNode::Video { class, id, src, children }),
        TokenRsH::Textarea(class, id) => Some(AstNode::Textarea { class, id, children }),
        TokenRsH::Element(tag, class, id, attrs, _) => Some(AstNode::Element { tag, class, id, attrs, children }),

        // Le premier champ des tokens de controle porte la condition brute.
        TokenRsH::If(condition, _) => Some(AstNode::If { condition, children }),
        TokenRsH::ElseIf(condition, _) => Some(AstNode::ElseIf { condition, children }),
        TokenRsH::Else(condition, _) => Some(AstNode::Else { condition, children }),
        TokenRsH::Match(condition, _) => Some(AstNode::Match { condition, children }),
        TokenRsH::Arm(pattern, _) => Some(AstNode::Arm { pattern, children }),
        TokenRsH::While(condition, _) => Some(AstNode::While { condition, children }),
        TokenRsH::For(condition, _) => Some(AstNode::For { condition, children }),
        TokenRsH::ForEach(condition, _) => Some(AstNode::ForEach { condition, children }),

        _ => None,
    }
}
