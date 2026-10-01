.app { display: flex; flex-direction: column; flex-grow: 1; background-color: #2b2d42; color: #f4f4f4; font-family: monospace; }
.tete { display: flex; flex-direction: row; align-items: center; gap: 22px; height: 64px; padding: 0 28px; flex-shrink: 0; background-color: #1b1b2f; border-bottom: 4px solid #000000; }
.logo { margin-right: 18px; color: #feae34; font-family: monospace; font-size: 24px; font-weight: 800; letter-spacing: 2px; }
.nav { color: #8b9bb4; font-family: monospace; font-size: 14px; }
.nav.actif { color: #f4f4f4; text-decoration: underline; }
.panier { margin-left: auto; padding: 8px 12px; background-color: #e43b44; border: 3px solid #000000; box-shadow: 3px 3px 0 0 #000000; font-family: monospace; font-size: 14px; font-weight: 800; }
.corps { display: flex; flex-direction: row; gap: 24px; padding: 24px 28px; }
.filtres { display: flex; flex-direction: column; gap: 6px; width: 170px; flex-shrink: 0; padding: 14px; background-color: #1b1b2f; border: 3px solid #000000; box-shadow: 4px 4px 0 0 #000000; }
.f-titre { margin-top: 6px; color: #feae34; font-family: monospace; font-size: 12px; font-weight: 800; }
.f { color: #c0cbdc; font-family: monospace; font-size: 13px; }
.f.actif { color: #63c74d; font-weight: 800; }
.grille { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 20px; flex-grow: 1; }
.produit { position: relative; display: flex; flex-direction: column; gap: 10px; padding: 12px; background-color: #3a3c5a; border: 3px solid #000000; box-shadow: 5px 5px 0 0 #000000; }
.badge { position: absolute; top: 10px; left: 10px; padding: 3px 6px; background-color: #63c74d; border: 2px solid #000000; color: #1b1b2f; font-family: monospace; font-size: 11px; font-weight: 800; }
.vitrine { display: flex; align-items: center; justify-content: center; height: 150px; background-color: #5a6988; border: 3px solid #000000; }
.sprite { display: flex; flex-direction: column; }
.px-ligne { display: flex; flex-direction: row; }
.px { width: 12px; height: 12px; }
.p-nom { font-family: monospace; font-size: 15px; font-weight: 800; }
.p-bas { display: flex; flex-direction: row; align-items: center; }
.p-prix { color: #feae34; font-family: monospace; font-size: 16px; font-weight: 800; }
.p-ajout { margin-left: auto; padding: 5px 8px; background-color: #0099db; border: 2px solid #000000; box-shadow: 2px 2px 0 0 #000000; font-family: monospace; font-size: 11px; font-weight: 800; }
.c-k { background-color: #1b1b2f; }
.c-r { background-color: #e43b44; }
.c-R { background-color: #a22633; }
.c-w { background-color: #f4f4f4; }
.c-y { background-color: #feae34; }
.c-Y { background-color: #c77b30; }
.c-b { background-color: #0099db; }
.c-B { background-color: #124e89; }
.c-g { background-color: #63c74d; }
.c-G { background-color: #3e8948; }
.c-s { background-color: #c0cbdc; }
.c-S { background-color: #8b9bb4; }
.c-p { background-color: #b55088; }
.c-P { background-color: #68386c; }
.c-o { background-color: #e4a672; }
