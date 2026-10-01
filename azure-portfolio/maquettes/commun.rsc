/* Commun aux maquettes : une fenetre de navigateur ou d'application. */
.ecran { display: flex; flex-direction: column; height: 100%; background-color: #ffffff; font-size: 14px; }
.chrome { display: flex; flex-direction: row; align-items: center; gap: 10px; height: 44px; padding: 0 16px; flex-shrink: 0; background-color: #e9e7e3; border-bottom: 1px solid #d3d0ca; }
.pastille { width: 12px; height: 12px; border-radius: 999px; background-color: #c9c5bd; }
.url { flex-grow: 1; margin-left: 16px; padding: 6px 14px; background-color: #ffffff; border-radius: 8px; color: #6b675f; font-size: 13px; }
.titre-fenetre { flex-grow: 1; text-align: center; color: #4a4741; font-size: 13px; font-weight: 600; }
