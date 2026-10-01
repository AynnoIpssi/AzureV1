/* Aides des apercus rsC (voir src/apercu.rs) : de quoi rendre visibles des
   boites que l'exemple seul laisserait transparentes. Elles passent AVANT
   l'exemple, qui l'emporte donc toujours. Prefixe ap- : jamais un nom
   d'exemple. */
.ap-zone { padding: 8px; background-color: #171614; border: 1px dashed #3a3530; border-radius: 6px; }
.ap-boite { padding: 8px 12px; background-color: #2a2620; border-radius: 6px; color: #e0cfb3; font-size: 13px; }
.ap-case { padding: 12px 14px; border: 1px solid #4a453e; border-radius: 6px; color: #d6d3cc; font-size: 13px; }
.ap-ligne { display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 10px; }
.ap-grand { height: 70px; }
.ap-ecran { display: flex; flex-direction: column; height: 200px; }
.ap-mot { color: #e0cfb3; font-weight: 700; }
