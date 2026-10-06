<!-- Tous les graphes de la librairie sur une page.
     Dessinee par azure-foundation/tests/libraire_graphes.rs. -->
<app.colonne>
<page titre="Graphes" description="Les modules chart-* d'azure-libraire.">
    <grid cols="4">
        <stat-spark label="Processeur" valeur="34 %" detail="+4 % en 1 min" tendance="hausse" valeurs="12, 18, 15, 22, 30, 26, 31, 28, 34"/>
        <stat-spark label="Memoire" valeur="412 Mo" valeurs="400, 402, 401, 405, 409, 408, 412" ton="succes"/>
        <stat-spark label="Erreurs" valeur="3" detail="-2 cette semaine" tendance="baisse" valeurs="9, 7, 8, 5, 6, 4, 3" ton="danger"/>
        <chart-gauge titre="Batterie" valeurs="72" unite=" %"/>
    <!grid>
    <grid cols="2">
        <chart-area titre="Puissance" detail="2 dernieres minutes" valeur="12.4 W" unite=" W" valeurs="8, 9, 8.5, 11, 14, 13, 12, 15, 19, 17, 14, 12, 11, 12.4, 13, 12, 10, 9, 11, 12.4" etiquettes="-20 s, , , , , -15 s, , , , , -10 s, , , , , -5 s, , , , 0"/>
        <chart-line titre="Processeur par app" legende="true" unite=" %" series="Docs: 4 6 5 9 12 8 7 6; Note: 2 3 8 14 9 5 4 3; Testeur: 1 1 2 2 20 26 18 6" etiquettes="10:00, 10:01, 10:02, 10:03, 10:04, 10:05, 10:06, 10:07"/>
        <chart-bars titre="Tests par jour" valeur="660" valeurs="420, 480, 510, 505, 590, 640, 660" etiquettes="Lun, Mar, Mer, Jeu, Ven, Sam, Dim"/>
        <chart-stack titre="Energie par paquet" legende="true" unite=" J" series="Tests: 12 14 11 16 13; Compilation: 30 4 6 28 5" etiquettes="#1, #2, #3, #4, #5"/>
        <chart-hbars titre="Les plus gourmands" unite=" mJ" valeurs="840, 610, 420, 180, 95" etiquettes="scroll_perf, texte_riche, ui_components, stockage_rss, router"/>
        <chart-donut titre="Repartition du processeur" valeurs="46, 27, 15, 12" etiquettes="azure-foundation, azure-stockage, azure-engine, autres" centre="35 s"/>
        <chart-pie titre="Resultats" valeurs="640, 14, 6" etiquettes="Reussis, Rates, Ignores" couleurs="#4ade80, #f87171, #8a877f"/>
        <chart-radar titre="Profil de deux passages" legende="true" series="Avant: 60 80 40 70 50 65; Apres: 75 60 55 85 45 70" etiquettes="Energie, Processeur, Memoire, Duree, Reveils, Disque"/>
        <chart-points.petit titre="Duree des tests" unite=" ms" valeurs="12, 40, 8, 95, 22, 31, 60, 14, 18, 77, 25, 9"/>
        <chart-line.petit titre="Sans donnees"/>
    <!grid>
<!page>
<!app>
