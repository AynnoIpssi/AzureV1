<container.app>

<container.haut>
    <button.marque#accueil>Azure Docs<!button>
    <text.marque-sous>documentation<!text>
    <container.haut-droite>
        <search.champ#q placeholder="Chercher : un mot, une page, rsc.flex.1…" value="{{q}}"/>
        <button.chercher#chercher>Chercher<!button>
    <!container>
<!container>

<container.corps>

<container.cote>
    <if.vue == "accueil"><button.cote-accueil.on#accueil>Accueil<!button><!if>
    <else><button.cote-accueil#accueil>Accueil<!button><!else>
    <for.s in sections>
        <if.s.actif == true>
            <button.cote-section.on#s-{{s.id}}>{{s.numero}}   {{s.titre}}<!button>
            <container.cote-pages>
            <for.p in s.pages>
                <if.p.actif == true><button.cote-page.on#p-{{s.id}}__{{p.id}}>{{p.titre}}<!button><!if>
                <else><button.cote-page#p-{{s.id}}__{{p.id}}>{{p.titre}}<!button><!else>
            <!for>
            <!container>
        <!if>
        <else>
            <button.cote-section#s-{{s.id}}>{{s.numero}}   {{s.titre}}<!button>
        <!else>
    <!for>
<!container>

<container.principal>
<match.vue>

<arm."accueil">
<container.large>
    <container.intro>
        <text.surtitre>Documentation<!text>
        <title1.intro-titre>Tout Azure, point par point.<!title1>
        <text.intro-texte>L'interface en rsH et rsC, la logique en Rust, et les daemons qui relient les apps entre elles : stockage, flux, appels, routeur, sécurité. Chaque exemple de code porte un identifiant, pour le retrouver depuis la recherche ou depuis l'IDE.<!text>
        <container.chiffres>
            <container.chiffre><text.chiffre-n>{{nb_sections}}<!text><text.chiffre-l>sections<!text><!container>
            <container.chiffre><text.chiffre-n>{{nb_pages}}<!text><text.chiffre-l>pages<!text><!container>
            <container.chiffre><text.chiffre-n>{{nb_exemples}}<!text><text.chiffre-l>exemples de code<!text><!container>
        <!container>
    <!container>

    <container.parcours>
        <text.bloc-titre>Par où commencer<!text>
        <container.parcours-ligne>
            <container.etape><text.etape-n>1<!text><text.etape-t>Comprendre Azure<!text><text.etape-d>Ce qu'est Azure, ce qu'on y trouve, et ce qui se passe quand une app s'ouvre.<!text><button.voile#p-demarrer__azure><!button><!container>
            <container.etape><text.etape-n>2<!text><text.etape-t>Créer une première app<!text><text.etape-d>Un manifeste, une page rsH, un style rsC, dix lignes de Rust.<!text><button.voile#p-demarrer__premiere-app><!button><!container>
            <container.etape><text.etape-n>3<!text><text.etape-t>Écrire l'interface<!text><text.etape-d>rsH pour la structure, rsC pour le style, les composants prêts à l'emploi.<!text><button.voile#s-rsh><!button><!container>
        <!container>
    <!container>

    <text.bloc-titre>Toutes les sections<!text>
    <container.grille>
    <for.s in sections>
        <container.carte>
            <container.carte-tete>
                <text.carte-num>{{s.numero}}<!text>
                <text.carte-compte>{{s.nb_pages}} pages · {{s.nb_exemples}} exemples<!text>
            <!container>
            <button.carte-titre#s-{{s.id}}>{{s.titre}}<!button>
            <text.carte-resume>{{s.resume}}<!text>
            <container.carte-pages>
            <for.p in s.pages>
                <button.carte-page#p-{{s.id}}__{{p.id}}>{{p.titre}}<!button>
            <!for>
            <!container>
        <!container>
    <!for>
    <!container>
<!container>
<!arm>

<arm."section">
<container.article>
    <container.fil>
        <button.fil-lien#accueil>Accueil<!button><text.fil-sep>/<!text><text.fil-ici>{{rubrique.titre}}<!text>
    <!container>
    <title1.titre>{{rubrique.titre}}<!title1>
    <text.resume>{{rubrique.resume}}<!text>
    <container.sommaire>
    <for.p in rubrique.pages>
        <container.ligne>
            <text.ligne-num>{{p_rang}}<!text>
            <container.ligne-texte>
                <text.ligne-titre>{{p.titre}}<!text>
                <text.ligne-resume>{{p.resume}}<!text>
            <!container>
            <text.ligne-compte>{{p.exemples}} ex.<!text>
            <button.voile#p-{{rubrique.id}}__{{p.id}}><!button>
        <!container>
    <!for>
    <!container>
<!container>
<!arm>

<arm."page">
<container.article>
    <container.fil>
        <button.fil-lien#accueil>Accueil<!button><text.fil-sep>/<!text>
        <button.fil-lien#s-{{doc.section}}>{{doc.section_titre}}<!button><text.fil-sep>/<!text>
        <text.fil-ici>{{doc.titre}}<!text>
    <!container>
    <title1.titre>{{doc.titre}}<!title1>
    <text.resume>{{doc.resume}}<!text>
    <container.filet><!container>

    <for.b in doc.blocs>
    <match.b.type>
    <arm."titre2"><title2.h2>{{b.texte}}<!title2><!arm>
    <arm."titre3"><title3.h3>{{b.texte}}<!title3><!arm>
    <arm."paragraphe"><text.p>{{b.texte}}<!text><!arm>
    <arm."liste">
        <container.liste>
        <for.i in b.items>
            <container.li><text.puce>–<!text><text.li-texte>{{i}}<!text><!container>
        <!for>
        <!container>
    <!arm>
    <arm."note">
        <container.note.{{b.genre}}>
            <text.note-titre>{{b.label}}<!text>
            <text.note-texte>{{b.texte}}<!text>
        <!container>
    <!arm>
    <arm."apercu">
        <container.apercu>
            <container.apercu-tete>
                <text.apercu-titre>APERÇU<!text>
                <text.apercu-legende>{{b.legende}}<!text>
                <text.apercu-note>rendu par le moteur d'Azure<!text>
            <!container>
            <container.apercu-cadre#apercu-{{b.n}}><!container>
        <!container>
    <!arm>
    <arm."demo">
        <container.demo>
            <container.demo-texte>
                <text.demo-titre>DÉMONSTRATION<!text>
                <text.demo-desc>{{b.texte}}<!text>
            <!container>
            <button.demo-ouvrir#demo-{{b.nom}}>Ouvrir<!button>
        <!container>
    <!arm>
    <arm."tableau">
        <container.tableau>
            <container.tr.entete>
            <for.c in b.entetes><text.th>{{c}}<!text><!for>
            <!container>
            <for.l in b.lignes>
                <container.tr>
                <for.c in l><text.td>{{c}}<!text><!for>
                <!container>
            <!for>
        <!container>
    <!arm>
    <arm."code">
        <container.code>
            <container.code-tete>
                <text.code-lang>{{b.langage}}<!text>
                <text.code-titre>{{b.titre}}<!text>
                <text.code-id>{{b.id}}<!text>
                <button.code-copier#copier-{{b.cle}}>Copier<!button>
            <!container>
            <container.code-corps><container.code-lignes>
            <for.l in b.lignes>
                <container.code-ligne><for.m in l><text.tk.{{m.k}}>{{m.v}}<!text><!for><!container>
            <!for>
            <!container><!container>
        <!container>
    <!arm>
    <!match>
    <!for>

    <container.voisines>
        <if.doc.precedente.existe == true>
            <container.voisine><text.voisine-l>Précédent<!text><text.voisine-t>{{doc.precedente.titre}}<!text><button.voile#p-{{doc.precedente.section}}__{{doc.precedente.id}}><!button><!container>
        <!if>
        <else><container.voisine-vide><!container><!else>
        <if.doc.suivante.existe == true>
            <container.voisine.droite><text.voisine-l>Suivant<!text><text.voisine-t>{{doc.suivante.titre}}<!text><button.voile#p-{{doc.suivante.section}}__{{doc.suivante.id}}><!button><!container>
        <!if>
    <!container>
<!container>
<!arm>

<arm."recherche">
<container.article>
    <container.fil>
        <button.fil-lien#accueil>Accueil<!button><text.fil-sep>/<!text><text.fil-ici>Recherche<!text>
    <!container>
    <title1.titre>Recherche<!title1>
    <if.q == "">
        <text.resume>Tape un mot, une phrase ou l'identifiant d'un exemple (par exemple rsc.flex.1) dans le champ en haut.<!text>
    <!if>
    <else>
        <text.resume>{{nb_resultats}} résultat(s) pour « {{q}} ».<!text>
    <!else>
    <container.resultats>
    <for.r in resultats>
        <container.resultat>
            <container.resultat-tete>
                <if.r.genre == "exemple"><text.resultat-genre.exemple>exemple<!text><text.resultat-id>{{r.id}}<!text><!if>
                <else><text.resultat-genre>page<!text><!else>
                <text.resultat-lieu>{{r.lieu}}<!text>
            <!container>
            <text.resultat-titre>{{r.titre}}<!text>
            <text.resultat-extrait>{{r.extrait}}<!text>
            <button.voile#p-{{r.cible}}><!button>
        <!container>
    <!for>
    <!container>
<!container>
<!arm>

<arm."erreur">
<container.article>
    <title1.titre>Contenu illisible<!title1>
    <text.erreur>{{erreur}}<!text>
<!container>
<!arm>

<!match>
<!container>

<!container>
<!container>
