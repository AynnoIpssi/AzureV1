#[derive(Debug, Clone, PartialEq)]
pub enum TokenRsH{
    //--------------------(Symbole :)------------------------->
    OpenTag, // = <
    CloseTag, // >
    EndTagMarker, // = <!
    Point, // = .
    OpenParenthesis, // = (
    CloseParenthesis, // = )
    OpenBrace, // = {
    CloseBrace, // = }
    OpenSquare, // = [
    CloseSquare, // = ]
    SimpleQuote, // = '
    DoubleQuote, // = "
    Plus, // = +
    Minus, // = -
    Asterisk, // = *
    Slash, // = /
    Unslash, // = \
    VerticalBar, // = |
    Comma, // = ,
    Colon, // = :
    SemiColon, // = ;
    ExclamationMark, // !, 
    //-----------------(Conditions/boucle :)---------------------->
    If(String, String),
    ElseIf(String, String),
    Else(String, String),
    Match(String, String),
    Arm(String, String),
    While(String, String),
    For(String, String),
    ForEach(String, String),
    //--------------------(Elements :)------------------------->
    Container(String, String), // = contaner : <container><!container>
    Title(String, String), // = title : <title><!title>
    Title1(String, String), // title 1, 2 , 3 = Title with diferente size and wieght
    Title2(String, String), 
    Title3(String, String), 
    Text(String, String), //text: <text><!text>
    RawText(String),  // text : text dans une balise
    Button(String, String),
    Image(String, String, String), // class, id, src (attribut src="...")
    Video(String, String, String), // class, id, src - src reste ignore pour l'instant (pas de decodage video, voir ui::models::video)
    Textarea(String, String), // = <textarea><!textarea> : zone de texte interactive
    /// Toute autre balise : composant (`<carte titre="...">`, `<checkbox/>`,
    /// `<input>`...). Nom, classes, id, attributs `nom="valeur"`, et `true`
    /// si elle se ferme elle-meme (`<divider/>`).
    Element(String, String, String, Vec<(String, String)>, bool),
    Unknown(String), // nom de balise vide ou invalide, ex: <>
}

/// Une position dans le texte source, pour rattacher une erreur a un endroit precis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn start() -> Self {
        Position { line: 1, column: 1 }
    }
}

/// Une valeur (token, noeud, ...) associee a sa position dans le source.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub pos: Position,
}

impl<T> Spanned<T> {
    pub fn new(value: T, pos: Position) -> Self {
        Spanned { value, pos }
    }
}