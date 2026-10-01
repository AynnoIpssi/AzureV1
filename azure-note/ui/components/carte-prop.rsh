<!-- La carte d'une propriete (`carte`, voir `proprietes::carte`) : ce qui
     est tape s'applique au clic suivant ; elle se ferme en cliquant ailleurs. -->
<container.fiche-prop>
    <container.fiche-tete>
        <text.fiche-icone>{{carte.icone}}<!text>
        <input.fiche-nom#carte-nom value="{{carte.nom}}" placeholder="Nom de la propriété" focus="{{carte.focus_nom}}"/>
        <tooltip texte="Fermer"><button.fiche-x#carte-fermer>×<!button><!tooltip>
    <!container>

    <text.fiche-titre>TYPE<!text>
    <container.fiche-types>
    <for.g in carte.types>
        <button.fiche-type.{{g.on}}#carte-genre-{{g.code}}>{{g.libelle}}<!button>
    <!for>
    <!container>

    <if.carte.a_options == true>
        <text.fiche-titre>OPTIONS<!text>
        <container.fiche-options>
        <for.o in carte.options>
            <container.fiche-option>
                <text.opt.{{o.teinte}}>{{o.nom}}<!text>
                <tooltip texte="Retirer l'option"><button.fiche-opt-x#carte-opt-suppr-{{o.i}}>×<!button><!tooltip>
            <!container>
        <!for>
        <!container>
        <input.fiche-champ#carte-opt placeholder="Nouvelle option puis Entrée"/>
    <!if>

    <if.carte.a_formule == true>
        <text.fiche-titre>FORMULE<!text>
        <input.fiche-champ.fiche-code#carte-formule value="{{carte.formule}}" placeholder="moyenne(enfants.avancement)" focus="{{carte.focus_formule}}"/>
        <text.fiche-aide>Entrée pour calculer · moyenne, somme, compte, min, max, si(…) · enfants.x = les sous-pages<!text>
    <!if>

    <container.fiche-pied>
        <if.carte.confirmer == true>
            <text.fiche-question>Supprimer « {{carte.nom}} » et ses valeurs ?<!text>
            <button.lien-discret#carte-suppr-non>Annuler<!button>
            <button.btn.danger#carte-suppr-oui>Supprimer<!button>
        <!if>
        <else><button.lien-discret.rouge#carte-suppr>Supprimer la propriété<!button><!else>
    <!container>
<!container>
