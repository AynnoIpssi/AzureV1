<!-- Ecran de l'app A : suit l'etat de la commande partagee par l'app B (parametre `etat`). -->
<container.page>
    <text>Commande<!text>
    <if.etat == "preparation"><text>En preparation<!text><!if>
    <elseif.etat == "livree"><text>Livree !<!text><!elseif>
    <else><text>En attente<!text><!else>
<!container>
