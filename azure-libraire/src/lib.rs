// La librairie d'Azure : tous les modules reutilisables, ranges par
// domaine. Un dossier par domaine :
//
// - `interface` : les modules du front (balises rsH + styles rsC) et les
//   themes qui les colorent ;
// - `back` : la logique reutilisable (hachage, chiffrement, compression,
//   encodage, dates, tableur, diff, lecture de code) ;
// - `service` : les methodes pretes a etre servies aux autres apps par
//   azure-service (un service par module du back).
//
// La librairie ne depend de rien. azure-foundation charge `interface`
// (voir INTERFACE.md) ; les crates d'Azure et les apps appellent `back`
// (voir BACK.md) ; `AzureApp::servir` branche un `service` (voir SERVICE.md).
pub mod back;
pub mod interface;
pub mod service;
