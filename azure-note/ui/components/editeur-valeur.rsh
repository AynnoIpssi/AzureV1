<!-- Le choix ouvert d'une valeur (`ed`, voir `proprietes::editeur`) :
     options en pastilles (+ creer), calendrier, ou pages a lier. -->
<container.choix-valeur>
<if.ed.genre == "date">
    <container.cal-tete>
        <text.cal-titre>{{ed.titre}}<!text>
        <tooltip texte="Mois précédent"><button.cal-nav#{{ed.prec}}>‹<!button><!tooltip>
        <tooltip texte="Mois suivant"><button.cal-nav#{{ed.suiv}}>›<!button><!tooltip>
    <!container>
    <container.cal-ligne>
        <text.cal-sem>lu<!text><text.cal-sem>ma<!text><text.cal-sem>me<!text><text.cal-sem>je<!text><text.cal-sem>ve<!text><text.cal-sem>sa<!text><text.cal-sem>di<!text>
    <!container>
    <for.s in ed.semaines>
        <container.cal-ligne>
        <for.d in s.jours><button.{{d.cl}}#{{d.id}}>{{d.j}}<!button><!for>
        <!container>
    <!for>
    <container.ed-pied>
        <button.lien-discret#{{ed.auj}}>Aujourd'hui<!button>
        <if.ed.a_date == true><button.lien-discret.rouge#{{ed.effacer}}>Effacer<!button><!if>
    <!container>
<!if>
<elseif.ed.genre == "relation">
    <input.ed-champ#creer-{{ed.fid}} value="{{ed.filtre}}" placeholder="Chercher une page puis Entrée" focus="true"/>
    <container.ed-liste>
    <for.x in ed.pages>
        <container.ed-ligne.{{x.on}}>
            <container.puce-page.t-{{x.teinte}}><!container>
            <button.ed-choix#{{x.choix}}>{{x.nom}}<!button>
            <tooltip texte="Ouvrir la page"><button.ed-aller#p-{{x.id}}>›<!button><!tooltip>
        <!container>
    <!for>
    <!container>
    <if.ed.a_pages == false><text.ed-vide>Aucune page<!text><!if>
<!elseif>
<else>
    <input.ed-champ#creer-{{ed.fid}} placeholder="{{ed.aide}}" focus="true"/>
    <if.ed.a_options == true>
    <container.ed-options>
    <for.o in ed.options><button.opt.choix.{{o.teinte}}.{{o.on}}#{{o.id}}>{{o.nom}}<!button><!for>
    <!container>
    <!if>
    <else><text.ed-vide>Tape un nom puis Entrée : l'option est créée.<!text><!else>
    <if.ed.retirer == true>
    <container.ed-pied><button.lien-discret#{{ed.aucune}}>Retirer<!button><!container>
    <!if>
<!else>
<!container>
