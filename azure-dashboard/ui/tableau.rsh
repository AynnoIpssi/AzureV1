<!-- Tableau de bord d'Azure. Donnees : `etat` (publie par azure-manager),
     `page` (apps, app, liens, services) et, sur la page d'une app, `app`. -->
<container.app>

<container.topbar>
    <container.logo><!container>
    <title3.brand>Azure Dashboard<!title3>
    <if.page == "apps"><button.nav.on#nav-apps>Apps<!button><!if>
    <elseif.page == "app"><button.nav.on#nav-apps>Apps<!button><!elseif>
    <else><button.nav#nav-apps>Apps<!button><!else>
    <if.page == "liens"><button.nav.on#nav-liens>Qui parle à qui<!button><!if>
    <else><button.nav#nav-liens>Qui parle à qui<!button><!else>
    <if.page == "services"><button.nav.on#nav-services>Services<!button><!if>
    <else><button.nav#nav-services>Services<!button><!else>
    <if.page == "evenements"><button.nav.on#nav-evenements>Événements<!button><!if>
    <else><button.nav#nav-evenements>Événements<!button><!else>
    <if.page == "journaux"><button.nav.on#nav-journaux>Journaux<!button><!if>
    <elseif.page == "journal"><button.nav.on#nav-journaux>Journaux<!button><!elseif>
    <else><button.nav#nav-journaux>Journaux<!button><!else>
    <if.page == "terminal"><button.nav.on#nav-terminal>Terminal<!button><!if>
    <else><button.nav#nav-terminal>Terminal<!button><!else>
    <text.pill>{{etat.nb_actives}} / {{etat.nb_apps}} apps actives<!text>
<!container>
<container.hr><!container>

<container.page>
<if.erreur != "">
    <container.alerte>
        <text.alerte-texte>{{erreur}}<!text>
        <button.alerte-x#erreur-fermer>Fermer<!button>
    <!container>
<!if>

<!-- ------------------------------------------------------------ apps -->
<if.page == "apps">
    <container.row>
        <title2.h>Apps<!title2>
        <button.btn.small#nav-terminal>Installer une app<!button>
    <!container>
    <text.lead>Chaque app se déclare dans son manifeste app.azure. Cliquez sur une app pour régler ses partages.<!text>
    <container.cards>
    <for.app in etat.apps>
        <container.card>
            <container.row>
                <title3.name>{{app.titre}}<!title3>
                <if.app.actif == true><text.badge.on>active<!text><!if>
                <else><text.badge.off>arrêtée<!text><!else>
                <if.app.nb_erreurs \> 0><text.badge.off>{{app.nb_erreurs}} erreur(s)<!text><!if>
            <!container>
            <text.muted>{{app.nom}} · id {{app.id}} · version {{app.version}}<!text>
            <for.partage in app.partages>
                <if.partage.public == true><text.line>partage « {{partage.flux}} » avec tout le monde<!text><!if>
                <elseif.partage.nb_autorises == 0><text.line>partage « {{partage.flux}} » avec personne<!text><!elseif>
                <else><text.line>partage « {{partage.flux}} » avec {{partage.autorises}}<!text><!else>
            <!for>
            <if.app.nb_partages == 0><if.app.nb_ecoutes == 0><text.muted>Ne partage ni n'écoute aucun flux.<!text><!if><!if>
            <for.ecoute in app.ecoutes>
                <text.line>écoute « {{ecoute.flux}} » de {{ecoute.source}}<!text>
            <!for>
            <for.methode in app.fournit>
                <if.methode.public == true><text.line>répond à « {{methode.methode}} » pour tout le monde<!text><!if>
                <elseif.methode.nb_autorises == 0><text.line>répond à « {{methode.methode}} » pour personne<!text><!elseif>
                <else><text.line>répond à « {{methode.methode}} » pour {{methode.autorises}}<!text><!else>
            <!for>
            <for.appel in app.utilise>
                <text.line>appelle « {{appel.methode}} » de {{appel.source}}<!text>
            <!for>
            <button.btn#app-{{app.nom}}>Régler<!button>
        <!container>
    <!for>
    <!container>
<!if>

<!-- ------------------------------------------------------- une app -->
<elseif.page == "app">
    <button.back#nav-apps>‹ Toutes les apps<!button>
    <container.row>
        <title2.h>{{app.titre}}<!title2>
        <if.app.actif == true><text.badge.on>active (pid {{app.pid}})<!text><!if>
        <else><text.badge.off>arrêtée<!text><!else>
    <!container>
    <text.muted>{{app.nom}} · id {{app.id}} · {{app.exe}}<!text>

    <title3.section>Compiler<!title3>
    <container.card.wide>
        <if.compilation == "en_cours">
            <container.row><text.badge.warn>compilation en cours<!text><text.muted>azure build {{app.nom}} --installer · une première compilation peut prendre plusieurs minutes<!text><!container>
        <!if>
        <else>
            <container.row>
                <button.btn.tight#compiler>Compiler et installer<!button>
                <if.compilation == "ok"><text.badge.on>compilée et installée<!text><!if>
                <elseif.compilation == "echec"><text.badge.off>échec<!text><!elseif>
            <!container>
            <if.compilation == ""><text.muted>Recompile l'app depuis ses sources (azure build {{app.nom}} --installer) puis la réinstalle. Relancez-la ensuite pour utiliser la nouvelle version.<!text><!if>
            <if.en_cours != ""><text.muted>« {{en_cours}} » tourne : attendez qu'elle finisse.<!text><!if>
        <!else>
        <if.compilation == "ok"><text.muted>Relancez l'app pour utiliser la nouvelle version.<!text><!if>
        <if.compilation != "">
            <if.compilation != "en_cours">
                <container.term.court>
                    <for.ligne in compilation_lignes>
                        <if.compilation == "ok"><text.term-out>{{ligne}}<!text><!if>
                        <else><text.term-err>{{ligne}}<!text><!else>
                    <!for>
                <!container>
                <button.btn.ghost.tight#nav-terminal>Toute la sortie dans le terminal ›<!button>
            <!if>
        <!if>
    <!container>

    <title3.section>Flux partagés<!title3>
    <if.app.nb_partages == 0><text.muted>Aucun.<!text><!if>
    <for.partage in app.partages>
        <container.card.wide>
            <container.row>
                <title3.name>{{partage.flux}}<!title3>
                <if.partage.public == true><text.badge.public>public<!text><!if>
                <else><text.badge.private>privé<!text><!else>
                <if.partage.modifie == true><text.badge.warn>modifié par le tableau de bord<!text><!if>
                <if.partage.stats.persistant == true><text.badge.public>persistant<!text><!if>
            <!container>
            <if.partage.stats.actif == true><text.stat>{{partage.stats.modifications}} modification(s) · {{partage.stats.ecoutes}} écoute(s) en cours<!text><!if>
            <else><text.muted>Pas partagé en ce moment.<!text><!else>
            <if.partage.nb_ecoutent == 0><text.muted>Aucune app ne déclare l'écouter.<!text><!if>
            <else><text.muted>Déclarent l'écouter : {{partage.ecoutent}}<!text><!else>
            <if.partage.public == true><text.line>Public : toutes les apps peuvent l'écouter.<!text><!if>
            <else>
                <text.label>Autorisées<!text>
                <if.partage.nb_autorises == 0><text.muted>personne<!text><!if>
                <container.chips>
                <for.cible in partage.autorises>
                    <container.chip>
                        <text.chiptext>{{cible}}<!text>
                        <button.x#retirer-{{partage_index}}-{{cible_index}}>Retirer<!button>
                    <!container>
                <!for>
                <!container>
                <text.label>Autoriser aussi<!text>
                <container.chips>
                <for.autre in partage.autres>
                    <button.add#autoriser-{{partage_index}}-{{autre_index}}>+ {{autre}}<!button>
                <!for>
                <!container>
            <!else>
            <container.row>
                <if.partage.public == true><button.btn#prive-{{partage_index}}>Rendre privé<!button><!if>
                <else><button.btn#public-{{partage_index}}>Rendre public<!button><!else>
                <if.partage.modifie == true><button.btn.ghost#reset-{{partage_index}}>Revenir au manifeste<!button><!if>
            <!container>
        <!container>
    <!for>

    <title3.section>Méthodes fournies<!title3>
    <if.app.nb_fournit == 0><text.muted>Aucune.<!text><!if>
    <for.methode in app.fournit>
        <container.card.wide>
            <container.row>
                <title3.name>{{methode.methode}}<!title3>
                <if.methode.public == true><text.badge.public>publique<!text><!if>
                <else><text.badge.private>privée<!text><!else>
                <if.methode.modifie == true><text.badge.warn>modifiée par le tableau de bord<!text><!if>
                <if.methode.stats.servie == true><text.badge.on>servie<!text><!if>
                <else><text.badge.off>pas servie<!text><!else>
            <!container>
            <text.muted>{{methode.description}}<!text>
            <text.stat>{{methode.stats.appels}} appel(s) · {{methode.stats.erreurs}} erreur(s) · {{methode.stats.delais}} délai(s) dépassé(s) · {{methode.stats.duree}} ms en moyenne<!text>
            <if.methode.stats.derniere_erreur != ""><text.error>Dernière erreur : {{methode.stats.derniere_erreur}}<!text><!if>
            <if.methode.nb_appelants == 0><text.muted>Aucune app ne déclare l'appeler.<!text><!if>
            <else><text.muted>Déclarent l'appeler : {{methode.appelants}}<!text><!else>
            <if.methode.public == true><text.line>Publique : toutes les apps peuvent l'appeler.<!text><!if>
            <else>
                <text.label>Autorisées<!text>
                <if.methode.nb_autorises == 0><text.muted>personne<!text><!if>
                <container.chips>
                <for.cible in methode.autorises>
                    <container.chip>
                        <text.chiptext>{{cible}}<!text>
                        <button.x#aretirer-{{methode_index}}-{{cible_index}}>Retirer<!button>
                    <!container>
                <!for>
                <!container>
                <text.label>Autoriser aussi<!text>
                <container.chips>
                <for.autre in methode.autres>
                    <button.add#aautoriser-{{methode_index}}-{{autre_index}}>+ {{autre}}<!button>
                <!for>
                <!container>
            <!else>
            <container.row>
                <if.methode.public == true><button.btn#aprive-{{methode_index}}>Rendre privée<!button><!if>
                <else><button.btn#apublic-{{methode_index}}>Rendre publique<!button><!else>
                <if.methode.modifie == true><button.btn.ghost#areset-{{methode_index}}>Revenir au manifeste<!button><!if>
            <!container>
        <!container>
    <!for>

    <title3.section>Méthodes utilisées<!title3>
    <if.app.nb_utilise == 0><text.muted>Aucune.<!text><!if>
    <for.appel in app.utilise>
        <text.line>« {{appel.methode}} » de {{appel.source}}<!text>
    <!for>

    <title3.section>Flux écoutés<!title3>
    <if.app.nb_ecoutes == 0><text.muted>Aucun.<!text><!if>
    <for.ecoute in app.ecoutes>
        <text.line>« {{ecoute.flux}} » de {{ecoute.source}}<!text>
    <!for>
    <title3.section>Sécurité<!title3>
    <container.card.wide>
        <container.row>
            <if.app.installee == true><text.badge.on>installée<!text><!if>
            <else><text.badge.warn>développement<!text><!else>
            <if.app.enfermee == true><text.badge.on>enfermée<!text><!if>
            <else><text.badge.off>non enfermée<!text><!else>
        <!container>
        <text.line>Permissions : {{app.permissions}}<!text>
        <if.app.empreinte != ""><text.muted>Empreinte de l'exécutable : {{app.empreinte}}…<!text><!if>
    <!container>

    <title3.section>Erreurs récentes<!title3>
    <if.app.nb_erreurs == 0><text.muted>Aucune.<!text><!if>
    <for.erreur in app.erreurs>
        <container.eventline><text.time>{{erreur.heure}}<!text><text.error>{{erreur.message}}<!text><!container>
    <!for>
    <button.link#journal-{{app.nom}}>Voir son journal<!button>

    <title3.section>Tâches de fond<!title3>
    <if.app.nb_services == 0><text.muted>Aucune.<!text><!if>
    <else><text.line>{{app.services}}<!text><!else>

    <title3.section>Oublier cette app<!title3>
    <container.card.wide>
        <text.muted>Azure oublie son nom, son identifiant et les réglages faits ici. Ses données ne sont pas effacées. Si elle se relance, elle sera enregistrée comme une nouvelle app.<!text>
        <if.confirmer_oubli == "oui">
            <text.line>Vraiment oublier {{app.titre}} ?<!text>
            <container.row>
                <button.btn.danger#oublier-oui>Oui, oublier<!button>
                <button.btn.ghost#oublier-non>Annuler<!button>
            <!container>
        <!if>
        <else><container.row><button.btn.danger#oublier>Oublier cette app<!button><!container><!else>
        <if.app.actif == true><text.muted>Elle tourne en ce moment : ferme-la d'abord.<!text><!if>
    <!container>
<!elseif>

<!-- ----------------------------------------------------------- liens -->
<elseif.page == "liens">
    <title2.h>Qui parle à qui<!title2>
    <text.lead>Chaque ligne : une app qui écoute le flux d'une autre, ou appelle une de ses méthodes, selon les manifestes.<!text>
    <for.lien in etat.liens>
        <container.link>
            <text.from>{{lien.vers}}<!text>
            <text.arrow>{{lien.verbe}} « {{lien.nom}} » de<!text>
            <text.to>{{lien.de}}<!text>
            <if.lien.autorise == true><text.badge.on>{{lien.statut}}<!text><!if>
            <else><text.badge.off>{{lien.statut}}<!text><!else>
        <!container>
    <!for>
<!elseif>

<!-- -------------------------------------------------------- services -->
<elseif.page == "services">
    <title2.h>Services d'arrière-plan<!title2>
    <if.etat.provider == true><text.lead>Surveillés par azure-provider : relancés automatiquement s'ils tombent.<!text><!if>
    <else><text.lead>azure-provider ne répond pas.<!text><!else>
    <for.service in etat.services>
        <container.link>
            <text.from>{{service.nom}}<!text>
            <if.service.pret == true><text.badge.on>{{service.etat}}<!text><!if>
            <else><text.badge.off>{{service.etat}}<!text><!else>
            <if.service.pret == true><text.muted>pid {{service.pid}} · {{service.relances}} relance(s)<!text><!if>
            <else><text.muted>{{service.relances}} relance(s)<!text><!else>
            <text.muted>{{service.message}}<!text>
            <button.btn.small#relancer-{{service.nom}}>Relancer<!button>
        <!container>
    <!for>
<!elseif>

<!-- ------------------------------------------------------ evenements -->
<elseif.page == "evenements">
    <title2.h>Événements<!title2>
    <text.lead>Ce que les apps signalent : erreurs, plantages, informations. Les plus récents d'abord.<!text>
    <if.etat.nb_evenements == 0><text.muted>Rien pour l'instant.<!text><!if>
    <for.ev in etat.evenements>
        <container.link>
            <text.time>{{ev.heure}}<!text>
            <text.from>{{ev.app}}<!text>
            <text.badge.{{ev.niveau}}>{{ev.niveau}}<!text>
            <text.line>{{ev.message}}<!text>
        <!container>
    <!for>
<!elseif>

<!-- -------------------------------------------------------- journaux -->
<elseif.page == "journaux">
    <title2.h>Journaux<!title2>
    <text.lead>La sortie des services d'arrière-plan et des apps lancées avec « azure run ».<!text>
    <title3.section>Services<!title3>
    <container.chips><for.s in etat.services><button.add#journal-{{s.nom}}>{{s.nom}}<!button><!for><!container>
    <title3.section>Apps<!title3>
    <container.chips><for.a in etat.apps><button.add#journal-{{a.nom}}>{{a.nom}}<!button><!for><!container>
<!elseif>
<elseif.page == "journal">
    <button.back#nav-journaux>‹ Journaux<!button>
    <container.row>
        <title2.h>{{journal}}<!title2>
        <button.btn.small#journal-actualiser>Actualiser<!button>
    <!container>
    <if.erreur_journal != ""><text.muted>{{erreur_journal}}<!text><!if>
    <container.logbox><for.ligne in lignes><text.logline>{{ligne}}<!text><!for><!container>
<!elseif>

<!-- -------------------------------------------------------- terminal -->
<elseif.page == "terminal">
    <container.row>
        <title2.h>Terminal<!title2>
        <button.btn.small#terminal-list>Apps installées<!button>
        <button.btn.ghost.tight#terminal-aide>Aide<!button>
        <button.btn.ghost.tight#terminal-effacer>Effacer<!button>
    <!container>
    <text.lead>Installer, désinstaller et lancer les apps avec la commande azure, exécutée par azure-manager (le tableau de bord, enfermé, n'a pas accès à vos dossiers).<!text>
    <container.term>
        <if.nb_terminal == 0><text.term-out>Tapez « install » suivi du dossier de l'app (celui qui contient app.azure), ou « aide », puis Entrée.<!text><!if>
        <for.l in terminal><text.term-{{l.genre}}>{{l.texte}}<!text><!for>
    <!container>
    <if.en_cours != ""><text.term-attente>En cours : {{en_cours}}… (la fenêtre reste utilisable)<!text><!if>
    <container.prompt>
        <text.term-signe>azure ›<!text>
        <input.term-saisie#commande placeholder="new meteo · build meteo --installer · run meteo" value="{{brouillon}}"/>
        <button.btn.small#terminal-executer>Exécuter<!button>
    <!container>
<!elseif>

<!container>
<!container>
