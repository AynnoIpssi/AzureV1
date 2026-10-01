<container.ecran>
    <container.chrome><container.pastille><!container><container.pastille><!container><container.pastille><!container><text.titre-fenetre>Explorateur de tests — App.Tests (NUnit)<!text><!container>
    <container.app>
        <container.gauche>
            <container.outils>
                <text.bouton>Tout exécuter<!text>
                <text.resume>42 tests<!text>
                <text.ok-compte>41 réussis<!text>
                <text.ko-compte>1 échoué<!text>
                <text.duree>18,4 s<!text>
            <!container>
            <container.arbre>
                <container.groupe><container.pt.ok><!container><text.g-nom>Tests unitaires<!text><text.g-n>28<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Connexion_MotDePasseVide_RefuseLaConnexion<!text><text.t-d>3 ms<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Panier_AjoutArticle_MetAJourLeTotal<!text><text.t-d>2 ms<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Validation_EmailInvalide_RetourneErreur<!text><text.t-d>1 ms<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Commande_SansArticle_NePeutPasEtreValidee<!text><text.t-d>2 ms<!text><!container>
                <container.groupe><container.pt.ko><!container><text.g-nom>Tests d'intégration — base SQL<!text><text.g-n>8<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Depot_Enregistrer_PuisRelire_RendLeMemeClient<!text><text.t-d>84 ms<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Depot_Supprimer_ClientAvecCommandes_EstRefuse<!text><text.t-d>61 ms<!text><!container>
                <container.test.choisi><container.pt.ko><!container><text.t-nom>Depot_Commande_TransactionAnnulee_SiStockInsuffisant<!text><text.t-d>112 ms<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Migration_BaseVide_CreeToutesLesTables<!text><text.t-d>240 ms<!text><!container>
                <container.groupe><container.pt.ok><!container><text.g-nom>Tests d'interface — Appium<!text><text.g-n>6<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Ecran_Connexion_IdentifiantsValides_OuvreAccueil<!text><text.t-d>3,2 s<!text><!container>
                <container.test><container.pt.ok><!container><text.t-nom>Ecran_Recherche_AfficheLesResultats<!text><text.t-d>2,7 s<!text><!container>
            <!container>
            <container.detail>
                <text.d-titre>Depot_Commande_TransactionAnnulee_SiStockInsuffisant<!text>
                <text.d-ligne>Expected: 5<!text>
                <text.d-ligne>  But was: 3<!text>
                <text.d-ligne.gris>à App.Tests.Integration.DepotCommandeTests.cs : ligne 87<!text>
            <!container>
        <!container>
        <container.droite>
            <text.d-label>APPIUM · ÉMULATEUR ANDROID<!text>
            <container.telephone>
                <container.ecran-tel>
                    <text.tel-titre>Connexion<!text>
                    <text.tel-label>Identifiant<!text><text.tel-champ>test.utilisateur<!text>
                    <text.tel-label>Mot de passe<!text><text.tel-champ>••••••••<!text>
                    <text.tel-bouton>Se connecter<!text>
                <!container>
            <!container>
            <container.journal>
                <text.j>› FindElement(AutomationId = "ChampIdentifiant")<!text>
                <text.j>› SendKeys("test.utilisateur")<!text>
                <text.j>› FindElement(AutomationId = "BoutonConnexion")<!text>
                <text.j.actif>› Click()  — en attente de l'écran Accueil…<!text>
            <!container>
        <!container>
    <!container>
<!container>
