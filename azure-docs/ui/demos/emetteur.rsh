<container.fen>
    <text.fen-surtitre>DÉMONSTRATION · ROUTEUR<!text>
    <title1.fen-titre>Émetteur<!title1>
    <text.fen-texte>Chaque action part vers la fenêtre Récepteur par le routeur : regarde-la changer.<!text>
    <text.etiquette>MESSAGE<!text>
    <container.ligne>
        <input.saisie#message placeholder="Écris quelque chose…"/>
        <button.principal#envoyer>Envoyer<!button>
    <!container>
    <text.etiquette>COULEUR DU PANNEAU<!text>
    <container.ligne>
        <button.pastille.sable#c-sable>Sable<!button>
        <button.pastille.sauge#c-sauge>Sauge<!button>
        <button.pastille.terracotta#c-terracotta>Terracotta<!button>
        <button.pastille.ardoise#c-ardoise>Ardoise<!button>
    <!container>
    <text.etiquette>COMPTEUR<!text>
    <container.ligne>
        <button.secondaire#moins>−1<!button>
        <button.secondaire#plus>+1<!button>
    <!container>
    <text.fen-pied>ctx.goto_view(recepteur, "/", donnees) à chaque clic.<!text>
<!container>
