// Le flux d'azure-service cote app : une app partage en temps reel les
// modifications d'un etat (et des evenements libres), les apps qu'elle a
// choisies les recoivent et s'en servent.
//
// ```text
// use azure_foundation::flux::{Flux, Value};
//
// // App B
// let mut panier = Flux::connect(APP_B)?.share("panier").to(&[APP_A]).open()?;
// panier.set("total", 42)?;
// panier.emit("paye", "carte")?;
//
// // App A, dans une fenetre : l'ecran suit le flux.
// let ecoute = Flux::connect(APP_A)?.listen(APP_B, "panier").path("total").start()?;
// AzureWindow::new("A")
//     .routes(routes).intra(intra)
//     .flux(ecoute, |ctx, event, etat| {
//         if event.touches("total") {
//             let total = etat.get("total").map(|v| v.to_string()).unwrap_or_default();
//             ctx.goto_route("panier", &[("total", &total)], "").ok();
//         }
//     })
//     .run();
// ```
pub use azure_service::flux::{Access, Batch, Change, Flux, FluxEvent, Listener, ListenerBuilder, Shared, ShareBuilder, StreamInfo, Value};

use crate::compiler::services::condition::ConditionValue;

/// Un etat de flux lisible par rsH : `Context::new().with_value("etat", flux::to_rsh(&etat))`
/// puis `{{etat.total}}`, `<for.item in etat.items>`. `Null` devient un
/// texte vide.
pub fn to_rsh(value: &Value) -> ConditionValue {
    match value {
        Value::Null => ConditionValue::Text(String::new()),
        Value::Bool(b) => ConditionValue::Bool(*b),
        Value::Int(i) => ConditionValue::Number(*i as f64),
        Value::Float(f) => ConditionValue::Number(*f),
        Value::Text(t) => ConditionValue::Text(t.clone()),
        Value::List(items) => ConditionValue::List(items.iter().map(to_rsh).collect()),
        Value::Map(map) => ConditionValue::Map(map.iter().map(|(k, v)| (k.clone(), to_rsh(v))).collect()),
    }
}

/// L'inverse de `to_rsh` : des donnees preparees pour rsH, envoyees telles
/// quelles a une autre app (reponse d'un `serve`). Un nombre entier devient
/// un `Int`.
pub fn from_rsh(value: &ConditionValue) -> Value {
    match value {
        ConditionValue::Bool(b) => Value::Bool(*b),
        ConditionValue::Number(n) if n.fract() == 0.0 && n.abs() < 9e15 => Value::Int(*n as i64),
        ConditionValue::Number(n) => Value::Float(*n),
        ConditionValue::Text(t) => Value::Text(t.clone()),
        ConditionValue::List(items) => Value::List(items.iter().map(from_rsh).collect()),
        ConditionValue::Map(map) => Value::Map(map.iter().map(|(k, v)| (k.clone(), from_rsh(v))).collect()),
    }
}
