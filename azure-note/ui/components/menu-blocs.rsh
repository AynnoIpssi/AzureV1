<!-- Le menu `/` : les genres de bloc (liste `menu` de src/page.rs). -->
<container.menu-blocs>
    <container.menu-tete>
        <text.menu-titre>INSÉRER UN BLOC<!text>
        <tooltip texte="Fermer"><button.menu-x#menu-fermer>×<!button><!tooltip>
    <!container>
    <container.menu-liste>
    <for.m in menu>
        <container.menu-ligne>
            <button.menu-item#menu-{{m.code}}>{{m.nom}}<!button>
            <text.menu-desc>{{m.desc}}<!text>
        <!container>
    <!for>
    <!container>
<!container>
