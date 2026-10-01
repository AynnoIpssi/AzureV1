<!-- « + Ajouter une propriété » : un clic sur un type la cree. -->
<container.types-prop>
    <text.fiche-titre>NOUVELLE PROPRIÉTÉ<!text>
    <container.types-liste>
    <for.g in types>
        <container.type-ligne>
            <text.type-icone>{{g.icone}}<!text>
            <button.type-nom#prop-nouveau-{{g.code}}>{{g.libelle}}<!button>
            <text.type-desc>{{g.desc}}<!text>
        <!container>
    <!for>
    <!container>
<!container>
