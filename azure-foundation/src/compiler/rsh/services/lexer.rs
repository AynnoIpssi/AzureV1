use crate::compiler::rsh::models::token::{Position, Spanned, TokenRsH};

// `<` demarre toujours une balise (voir la boucle principale de `tokenize`)
// et ne peut donc jamais apparaitre litteralement dans du texte affiche -
// meme limitation que le HTML, resolue de la meme facon : quelques entites
// nommees, decodees apres coup sur le texte brut deja capture (aucune
// n'introduit elle-meme un `<`/`&` qui pourrait rouvrir une balise ou se
// faire redecoder deux fois - `&amp;` est applique en dernier expres pour
// ca, comme le veut la regle HTML habituelle).
fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

fn advance(line: &mut usize, column: &mut usize, c: char) {
    if c == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

// Les balises de controle (condition/boucle) portent une expression Rust
// arbitraire apres leur '.', pas un simple nom de classe : `<if.count != 0>`,
// `<for.item in liste>`, `<while.has_next()>`. On les distingue des balises
// d'element (dont le '.' introduit un nom de classe CSS strict) pour choisir
// la bonne regle de capture. `arm` (le motif d'une branche de `<match>`, voir
// `TokenRsH::Arm`) en fait partie pour la meme raison : un motif peut etre un
// litteral texte entre guillemets, plusieurs valeurs separees par `|`, ou `_`
// (le "catch-all" - voir `condition::matches_pattern`), aucun de ces
// caracteres n'etant valide dans un nom de classe CSS strict.
//
// Limite connue, avec echappement : l'expression est capturee brute jusqu'au
// premier '>' NON echappe, qui est aussi le caractere qui ferme la balise.
// `<`/`>` seuls (comparaisons "plus petit que"/"plus grand que") sont donc
// ambigus avec la fermeture de balise - meme limitation que le HTML, resolue
// de la meme facon : `\<`/`\>` pour un caractere litteral (voir plus bas),
// ou `!=`/`==`/`&&`/`||`/un appel de methode pour l'eviter completement.
fn is_control_flow_tag(word: &str) -> bool {
    matches!(word, "if" | "elseif" | "else" | "match" | "arm" | "while" | "for" | "foreach")
}

pub fn tokenize(input: &str) -> Vec<Spanned<TokenRsH>> {
    let mut chars = input.chars().peekable();
    let mut result = Vec::new();
    let mut word = String::new();
    let mut class_name = String::new();
    let mut id_name = String::new();
    let mut src = String::new();
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut self_closing = false;

    let mut line = 1usize;
    let mut column = 1usize;

    while let Some(c) = chars.next() {
        // Tous les tokens produits pendant cette iteration (une balise entiere,
        // ou un bloc de texte) partagent la position ou l'iteration a commence.
        let pos = Position { line, column };
        advance(&mut line, &mut column, c);

        // Commentaire `<!-- ... -->`, comme en HTML : repere AVANT `<!`
        // (fermeture de balise), qu'il prolonge. Ne produit aucun token. Un
        // commentaire jamais ferme court jusqu'a la fin du fichier, comme en
        // HTML.
        if c == '<' && chars.clone().take(3).eq("!--".chars()) {
            for skipped in chars.by_ref().take(3) {
                advance(&mut line, &mut column, skipped);
            }
            let mut dashes = 0;
            for skipped in chars.by_ref() {
                advance(&mut line, &mut column, skipped);
                match skipped {
                    '-' => dashes += 1,
                    '>' if dashes >= 2 => break,
                    _ => dashes = 0,
                }
            }
            continue;
        }

        if c == '<' {
            let next_char = chars.peek();

            if next_char == Some(&'!') {
                if let Some(bang) = chars.next() {
                    advance(&mut line, &mut column, bang);
                }
                result.push(Spanned::new(TokenRsH::EndTagMarker, pos));
            } else {
                result.push(Spanned::new(TokenRsH::OpenTag, pos));
            }

            while let Some(&next) = chars.peek() {
                // alphanumerique : les noms de balise comme "title1"/"title2"
                // contiennent un chiffre ; `-`/`_` pour les composants
                // (`<list-item>`).
                if next.is_alphanumeric() || (!word.is_empty() && (next == '-' || next == '_')) {
                    word.push(next);
                    chars.next();
                    advance(&mut line, &mut column, next);
                } else {
                    break;
                }
            }

            if is_control_flow_tag(&word) {
                // Expression brute : tout ce qui suit le '.' jusqu'au '>' qui
                // ferme la balise, sans restriction de caracteres (espaces,
                // operateurs, parentheses... sont valides ici).
                if chars.peek() == Some(&'.') {
                    chars.next();
                    advance(&mut line, &mut column, '.');
                    while let Some(&next) = chars.peek() {
                        if next == '>' {
                            break;
                        }
                        if next == '\\' {
                            // Echappement pour un '<'/'>' litteral dans la
                            // condition (comparaison), qui sinon serait
                            // confondu avec le '>' qui ferme la balise - voir
                            // la doc ci-dessus. Pas une syntaxe d'echappement
                            // generale : un backslash suivi d'autre chose
                            // reste un backslash litteral.
                            chars.next();
                            advance(&mut line, &mut column, next);
                            match chars.peek() {
                                Some(&escaped) if escaped == '<' || escaped == '>' => {
                                    class_name.push(escaped);
                                    chars.next();
                                    advance(&mut line, &mut column, escaped);
                                }
                                _ => class_name.push('\\'),
                            }
                            continue;
                        }
                        class_name.push(next);
                        chars.next();
                        advance(&mut line, &mut column, next);
                    }
                }
            } else {
                // Plusieurs classes possibles (`<text.note.warning>`, comme
                // `class="note warning"` en HTML) : gardees separees par un
                // espace dans `class_name`.
                while chars.peek() == Some(&'.') {
                    chars.next();
                    advance(&mut line, &mut column, '.');
                    if !class_name.is_empty() {
                        class_name.push(' ');
                    }
                    // '-' accepte en plus de alphanumerique/'_' : convention
                    // CSS courante pour les noms de classe ("primary-button").
                    // `{{...}}` : classe calculee (`<text.badge.{{statut}}>`,
                    // voir `condition::interpolate`).
                    while let Some(&next) = chars.peek() {
                        if next.is_alphanumeric() || next == '_' || next == '-' {
                            class_name.push(next);
                            chars.next();
                            advance(&mut line, &mut column, next);
                        } else if next == '{' && chars.clone().nth(1) == Some('{') {
                            let mut previous = ' ';
                            for c in chars.by_ref() {
                                advance(&mut line, &mut column, c);
                                class_name.push(c);
                                if c == '}' && previous == '}' {
                                    break;
                                }
                                previous = c;
                            }
                        } else {
                            break;
                        }
                    }
                }

                if chars.peek() == Some(&'#') {
                    chars.next();
                    advance(&mut line, &mut column, '#');
                    while let Some(&next) = chars.peek() {
                        if next.is_alphanumeric() || next == '_' || next == '-' {
                            id_name.push(next);
                            chars.next();
                            advance(&mut line, &mut column, next);
                        } else if next == '{' && chars.clone().nth(1) == Some('{') {
                            // `#ouvrir-{{app.nom}}` : variable remplacee a la
                            // construction (voir `condition::interpolate`),
                            // gardee telle quelle jusqu'a `}}`.
                            let mut previous = ' ';
                            for c in chars.by_ref() {
                                advance(&mut line, &mut column, c);
                                id_name.push(c);
                                if c == '}' && previous == '}' {
                                    break;
                                }
                                previous = c;
                            }
                        } else {
                            break;
                        }
                    }
                }

                // Attributs `nom="valeur"` (seul `src`, sur <image>/<video>,
                // en lit un pour l'instant - voir TokenRsH::Image/Video) :
                // zero ou plusieurs, separes par des espaces, avant le '>'
                // qui ferme la balise. Non reconnus pour les balises de
                // controle (leur '.' capture deja une expression brute
                // jusqu'au '>', il n'y a pas de place pour ca).
                loop {
                    while matches!(chars.peek(), Some(&s) if s == ' ' || s == '\t') {
                        if let Some(ws) = chars.next() {
                            advance(&mut line, &mut column, ws);
                        }
                    }
                    match chars.peek() {
                        Some(&c) if c.is_alphabetic() => {
                            let mut attr_name = String::new();
                            while let Some(&next) = chars.peek() {
                                if next.is_alphanumeric() || next == '-' || next == '_' {
                                    attr_name.push(next);
                                    chars.next();
                                    advance(&mut line, &mut column, next);
                                } else {
                                    break;
                                }
                            }
                            if chars.peek() == Some(&'=')
                                && let Some(eq) = chars.next() {
                                    advance(&mut line, &mut column, eq);
                                }
                            let mut attr_value = String::new();
                            if chars.peek() == Some(&'"') {
                                if let Some(q) = chars.next() {
                                    advance(&mut line, &mut column, q);
                                }
                                while let Some(&next) = chars.peek() {
                                    if next == '"' {
                                        break;
                                    }
                                    attr_value.push(next);
                                    chars.next();
                                    advance(&mut line, &mut column, next);
                                }
                                if let Some(q) = chars.next() {
                                    advance(&mut line, &mut column, q);
                                }
                            }
                            if attr_name == "src" {
                                src.push_str(&attr_value);
                            }
                            attrs.push((attr_name, decode_entities(&attr_value)));
                        }
                        _ => break,
                    }
                }
                // `<divider/>` : balise qui se ferme elle-meme.
                if chars.peek() == Some(&'/') && chars.clone().nth(1) == Some('>') {
                    chars.next();
                    advance(&mut line, &mut column, '/');
                    self_closing = true;
                }
            }

            match word.as_str() {
                "title" => result.push(Spanned::new(TokenRsH::Title(class_name.clone(), id_name.clone()), pos)),
                "title1" => result.push(Spanned::new(TokenRsH::Title1(class_name.clone(), id_name.clone()), pos)),
                "title2" => result.push(Spanned::new(TokenRsH::Title2(class_name.clone(), id_name.clone()), pos)),
                "title3" => result.push(Spanned::new(TokenRsH::Title3(class_name.clone(), id_name.clone()), pos)),
                "container" => result.push(Spanned::new(TokenRsH::Container(class_name.clone(), id_name.clone()), pos)),
                "text" => result.push(Spanned::new(TokenRsH::Text(class_name.clone(), id_name.clone()), pos)),
                "button" => result.push(Spanned::new(TokenRsH::Button(class_name.clone(), id_name.clone()), pos)),
                "image" => result.push(Spanned::new(TokenRsH::Image(class_name.clone(), id_name.clone(), src.clone()), pos)),
                "video" => result.push(Spanned::new(TokenRsH::Video(class_name.clone(), id_name.clone(), src.clone()), pos)),
                "textarea" => result.push(Spanned::new(TokenRsH::Textarea(class_name.clone(), id_name.clone()), pos)),
                // Le champ "id" des tokens de controle n'est pas utilise :
                // l'expression complete est deja dans class_name.
                "if" => result.push(Spanned::new(TokenRsH::If(class_name.clone(), id_name.clone()), pos)),
                "elseif" => result.push(Spanned::new(TokenRsH::ElseIf(class_name.clone(), id_name.clone()), pos)),
                "else" => result.push(Spanned::new(TokenRsH::Else(class_name.clone(), id_name.clone()), pos)),
                "match" => result.push(Spanned::new(TokenRsH::Match(class_name.clone(), id_name.clone()), pos)),
                "arm" => result.push(Spanned::new(TokenRsH::Arm(class_name.clone(), id_name.clone()), pos)),
                "for" => result.push(Spanned::new(TokenRsH::For(class_name.clone(), id_name.clone()), pos)),
                "foreach" => result.push(Spanned::new(TokenRsH::ForEach(class_name.clone(), id_name.clone()), pos)),
                "while" => result.push(Spanned::new(TokenRsH::While(class_name.clone(), id_name.clone()), pos)),
                "" => result.push(Spanned::new(TokenRsH::Unknown(String::new()), pos)),
                // Toute autre balise : un composant (voir `compiler::components`),
                // resolu a la construction de l'interface.
                other => result.push(Spanned::new(TokenRsH::Element(other.to_string(), class_name.clone(), id_name.clone(), std::mem::take(&mut attrs), self_closing), pos)),
            }
            word.clear();
            class_name.clear();
            id_name.clear();
            src.clear();
            attrs.clear();
            self_closing = false;

            if let Some(close) = chars.next() {
                advance(&mut line, &mut column, close);
                if close == '>' {
                    result.push(Spanned::new(TokenRsH::CloseTag, pos));
                }
            }
        } else {
            let mut text_content = String::new();
            text_content.push(c);

            while let Some(&next) = chars.peek() {
                if next == '<' {
                    break;
                }

                text_content.push(next);
                chars.next();
                advance(&mut line, &mut column, next);
            }

            result.push(Spanned::new(TokenRsH::RawText(decode_entities(&text_content)), pos));
        }
    }
    result
}
