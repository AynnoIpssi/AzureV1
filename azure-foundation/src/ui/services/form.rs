// Les valeurs des champs d'une page (`<input#email>`, `<checkbox#cgu>`,
// `<radio name="taille">`...), lues par l'app au moment d'un clic (voir
// `WindowContext::value`).
use crate::ui::models::control::ControlKind;
use crate::ui::models::ui_node::UiNode;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    Bool(bool),
    Number(f64),
}

impl FieldValue {
    /// La valeur en texte (`"true"`, `"42"`...).
    pub fn as_text(&self) -> String {
        match self {
            FieldValue::Text(t) => t.clone(),
            FieldValue::Bool(b) => b.to_string(),
            FieldValue::Number(n) if n.fract() == 0.0 => format!("{}", *n as i64),
            FieldValue::Number(n) => n.to_string(),
        }
    }
}

/// Les champs qui ont un `#id` (et les groupes de radios, par `name`).
pub fn form_values(nodes: &[UiNode]) -> BTreeMap<String, FieldValue> {
    let mut values = BTreeMap::new();
    collect(nodes, &mut values);
    values
}

fn collect(nodes: &[UiNode], values: &mut BTreeMap<String, FieldValue>) {
    for node in nodes {
        match node {
            UiNode::TextArea(area) if !area.id.is_empty() => {
                // Texte riche : au format d'echange (voir `ui::models::rich`).
                let text = area.rich_value().unwrap_or_else(|| area.text.clone());
                values.insert(area.id.clone(), FieldValue::Text(text));
            }
            UiNode::Control(c) => {
                let value = match c.kind {
                    ControlKind::Checkbox | ControlKind::Switch => Some(FieldValue::Bool(c.checked)),
                    ControlKind::Slider | ControlKind::Progress | ControlKind::Rating => Some(FieldValue::Number(c.value)),
                    ControlKind::Select | ControlKind::Segmented => c.selected_value().map(|v| FieldValue::Text(v.to_string())),
                    ControlKind::Radio => {
                        if c.checked && !c.name.is_empty() {
                            values.insert(c.name.clone(), FieldValue::Text(c.radio_value.clone()));
                        }
                        Some(FieldValue::Bool(c.checked))
                    }
                };
                if let (Some(value), false) = (value, c.id.is_empty()) {
                    values.insert(c.id.clone(), value);
                }
            }
            UiNode::Container(container) => collect(&container.children, values),
            _ => {}
        }
    }
}
