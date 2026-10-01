// Le hub du daemon, sans socket : les envois arrivent dans des canaux.
use azure_core::models::wire::Reader;
use azure_service::flux::hub::Hub;
use azure_service::flux::path::Path;
use azure_service::flux::protocol::*;
use azure_service::flux::{Access, Change, Filter, Value};
use std::sync::mpsc::{sync_channel, Receiver};

const B: u32 = 2;
const A: u32 = 1;
const C: u32 = 3;

#[derive(Debug, PartialEq)]
enum Got {
    Snapshot(u64, String),
    Update(u64, Vec<Change>),
    Closed,
    Denied(String),
}

fn next(rx: &Receiver<Vec<u8>>) -> Option<Got> {
    let frame = rx.try_recv().ok()?;
    let mut r = Reader::new(&frame);
    Some(match r.u8().unwrap() {
        PUSH_SNAPSHOT => Got::Snapshot(r.u64().unwrap(), Value::read(&mut r).unwrap().to_string()),
        PUSH_UPDATE => Got::Update(r.u64().unwrap(), read_changes(&mut r).unwrap()),
        PUSH_CLOSED => Got::Closed,
        PUSH_DENIED => Got::Denied(r.str().unwrap()),
        other => panic!("envoi {other}"),
    })
}

fn listen(hub: &mut Hub, app: u32, filter: Filter) -> Result<Receiver<Vec<u8>>, String> {
    let (tx, rx) = sync_channel(64);
    hub.listen(app, B, "panier", filter, tx)?;
    Ok(rx)
}

#[test]
fn a_late_listener_gets_the_state_then_the_changes() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Public).unwrap();
    hub.publish(B, "panier", &[Change::set("total", 10), Change::set("client", "Ana")]).unwrap();

    let rx = listen(&mut hub, A, Filter::default()).unwrap();
    assert_eq!(next(&rx), Some(Got::Snapshot(1, r#"{"client": "Ana", "total": 10}"#.into())));

    let seq = hub.publish(B, "panier", &[Change::set("total", 12), Change::event("paye", "carte")]).unwrap();
    assert_eq!(seq, 2);
    assert_eq!(next(&rx), Some(Got::Update(2, vec![Change::set("total", 12), Change::event("paye", "carte")])));
    assert_eq!(next(&rx), None);
}

#[test]
fn each_listener_only_receives_what_it_filtered() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Public).unwrap();
    hub.publish(B, "panier", &[Change::set("total", 10), Change::set("client", "Ana")]).unwrap();
    let total_only = listen(&mut hub, A, Filter { paths: vec![Path::pattern("total").unwrap()], events: vec!["paye".into()] }).unwrap();
    let everything = listen(&mut hub, C, Filter::default()).unwrap();
    assert_eq!(next(&total_only), Some(Got::Snapshot(1, r#"{"total": 10}"#.into())));
    next(&everything);

    hub.publish(B, "panier", &[Change::set("client", "Bo"), Change::event("vu", 1)]).unwrap();
    assert_eq!(next(&total_only), None, "rien qui la concerne : rien d'envoye");
    assert_eq!(next(&everything), Some(Got::Update(2, vec![Change::set("client", "Bo"), Change::event("vu", 1)])));

    hub.publish(B, "panier", &[Change::set("client", "Cy"), Change::set("total", 11), Change::event("paye", true)]).unwrap();
    assert_eq!(next(&total_only), Some(Got::Update(3, vec![Change::set("total", 11), Change::event("paye", true)])));
}

#[test]
fn a_bad_change_applies_nothing() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Public).unwrap();
    hub.publish(B, "panier", &[Change::set("n", 1)]).unwrap();
    let rx = listen(&mut hub, A, Filter::default()).unwrap();
    next(&rx);

    let error = hub.publish(B, "panier", &[Change::set("ok", 1), Change::push("n", 2)]).unwrap_err();
    assert!(error.contains("pas une liste"));
    assert_eq!(next(&rx), None);
    let (seq, state) = hub.share(B, "panier", Access::Public).unwrap();
    assert_eq!((seq, state.to_string()), (1, r#"{"n": 1}"#.to_string()), "ni 'ok' ni nouveau numero");
    assert!(hub.publish(A, "panier", &[Change::set("x", 1)]).unwrap_err().contains("pas partage"), "A n'a pas de flux 'panier'");
}

#[test]
fn only_the_apps_chosen_by_b_can_listen() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Apps(vec![A])).unwrap();
    assert!(listen(&mut hub, A, Filter::default()).is_ok());
    assert!(listen(&mut hub, C, Filter::default()).unwrap_err().contains("ne partage pas 'panier' avec l'app 3"));
    // Son proprietaire peut toujours l'ecouter.
    let (tx, _rx) = sync_channel(4);
    assert!(hub.listen(B, B, "panier", Filter::default(), tx).is_ok());

    let names = |app| hub.streams(app).into_iter().map(|s| (s.owner, s.name)).collect::<Vec<_>>();
    assert_eq!(names(A), [(B, "panier".to_string())]);
    assert!(names(C).is_empty());
}

#[test]
fn removing_an_app_from_the_access_cuts_its_listening() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Apps(vec![A, C])).unwrap();
    let a = listen(&mut hub, A, Filter::default()).unwrap();
    let c = listen(&mut hub, C, Filter::default()).unwrap();
    next(&a);
    next(&c);

    hub.share(B, "panier", Access::Apps(vec![A])).unwrap();
    assert_eq!(next(&c), Some(Got::Denied("L'app 2 ne partage plus 'panier' avec l'app 3".into())));
    hub.publish(B, "panier", &[Change::set("x", 1)]).unwrap();
    assert!(matches!(next(&a), Some(Got::Update(1, _))));
    assert_eq!(next(&c), None);
    assert_eq!(hub.listeners(B, "panier"), 1);
}

#[test]
fn listening_before_b_shares_waits_for_it() {
    let mut hub = Hub::new();
    let early = listen(&mut hub, A, Filter::default()).unwrap();
    let refused = listen(&mut hub, C, Filter::default()).unwrap();
    assert_eq!(next(&early), None, "en attente");

    hub.share(B, "panier", Access::Apps(vec![A])).unwrap();
    assert_eq!(next(&early), Some(Got::Snapshot(0, "{}".into())));
    assert!(matches!(next(&refused), Some(Got::Denied(_))));
}

#[test]
fn closing_a_stream_keeps_its_listeners_waiting() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Public).unwrap();
    hub.publish(B, "panier", &[Change::set("x", 1)]).unwrap();
    let rx = listen(&mut hub, A, Filter::default()).unwrap();
    next(&rx);

    hub.close(B, "panier").unwrap();
    assert_eq!(next(&rx), Some(Got::Closed));
    assert!(hub.streams(A).is_empty());
    // Partage a nouveau : etat vide, l'ecoute repart.
    hub.share(B, "panier", Access::Public).unwrap();
    assert_eq!(next(&rx), Some(Got::Snapshot(0, "{}".into())));
}

#[test]
fn a_listener_that_does_not_read_is_cut_without_blocking_others() {
    let mut hub = Hub::new();
    hub.share(B, "panier", Access::Public).unwrap();
    let (tx, _slow) = sync_channel(2);
    hub.listen(A, B, "panier", Filter::default(), tx).unwrap();
    let fast = listen(&mut hub, C, Filter::default()).unwrap();
    for i in 0..10 {
        hub.publish(B, "panier", &[Change::set("i", i)]).unwrap();
    }
    assert_eq!(hub.listeners(B, "panier"), 1, "l'ecoute lente est coupee");
    let received = std::iter::from_fn(|| next(&fast)).count();
    assert_eq!(received, 11, "etat + 10 modifications");
}
