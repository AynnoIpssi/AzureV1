<!-- La fenetre Doc d'Azure Note (src/doc.rs). Le contenu vient d'Azure Docs.
     vue = "resultats" (resultats, nb), "page" (doc, vise), "erreur" (erreur). -->
<container.app>

<container.haut>
    <text.source>AZURE DOCS<!text>
    <search.champ#q placeholder="Chercher dans la doc…" value="{{q}}"/>
    <button.chercher#chercher>Chercher<!button>
<!container>

<container.page>

<match.vue>
<arm."erreur">
    <container.erreur>
        <text.erreur-titre>Doc indisponible<!text>
        <text.erreur-texte>{{erreur}}<!text>
        <button.btn#reessayer>Réessayer<!button>
    <!container>
<!arm>

<arm."resultats">
    <if.q == "">
        <text.lead>Tape un mot, une page ou un identifiant d'exemple (rsc.flex.1).<!text>
    <!if>
    <elseif.nb == 0>
        <text.lead>Rien trouvé pour « {{q}} ».<!text>
    <!elseif>
    <else>
        <text.lead>{{nb}} résultat(s) pour « {{q}} »<!text>
        <for.r in resultats>
            <container.res>
                <container.res-tete>
                    <text.res-genre.g-{{r.genre}}>{{r.genre}}<!text>
                    <button.res-titre#r-{{r.i}}>{{r.titre}}<!button>
                <!container>
                <text.res-lieu>{{r.lieu}}<!text>
                <text.res-extrait>{{r.extrait}}<!text>
            <!container>
        <!for>
    <!else>
<!arm>

<arm."page">
    <container.fil>
        <button.retour#retour>Résultats<!button>
        <text.fil-section>{{doc.section}}<!text>
        <button.ouvrir#ouvrir-docs>Ouvrir dans Azure Docs<!button>
    <!container>
    <title1.titre>{{doc.titre}}<!title1>
    <text.resume>{{doc.resume}}<!text>
    <for.b in vise>
        <text.vise-titre>L'EXEMPLE CHERCHÉ<!text>
        <container.code.vise>
            <container.code-tete>
                <text.code-lang>{{b.langage}}<!text>
                <text.code-titre>{{b.titre}}<!text>
                <text.code-id>{{b.id}}<!text>
                <button.code-copier#vu-copier-{{b.cle}}>Copier<!button>
            <!container>
            <container.code-corps><container.code-lignes>
            <for.l in b.lignes>
                <container.code-ligne><for.m in l><text.tk.{{m.k}}>{{m.v}}<!text><!for><!container>
            <!for>
            <!container><!container>
        <!container>
        <text.vise-titre>TOUTE LA PAGE<!text>
    <!for>
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
<!arm>
<!match>

<!container>
<!container>
