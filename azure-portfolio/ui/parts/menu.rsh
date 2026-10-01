<!-- L'en-tete, commun a toutes les pages. `page` : la page ouverte.
     A remplir : votre nom. -->
<container.haut>
    <text.marque>Yoann Fayolle<!text>
    <if.page == "presentation"><button.nav.on#nav-presentation>Présentation<!button><!if>
    <else><button.nav#nav-presentation>Présentation<!button><!else>
    <if.page == "projets"><button.nav.on#nav-projets>Projets<!button><!if>
    <else><button.nav#nav-projets>Projets<!button><!else>
    <if.page == "azure"><button.nav.on#nav-azure>Azure<!button><!if>
    <else><button.nav#nav-azure>Azure<!button><!else>
    <if.page == "etudes"><button.nav.on#nav-etudes>Parcours<!button><!if>
    <else><button.nav#nav-etudes>Parcours<!button><!else>
    <if.page == "veille"><button.nav.on#nav-veille>Veille<!button><!if>
    <else><button.nav#nav-veille>Veille<!button><!else>
<!container>
