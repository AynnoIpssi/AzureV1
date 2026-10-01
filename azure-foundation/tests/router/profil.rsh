<!-- Page de test de RouteTable::view : les parametres du chemin arrivent dans les conditions. -->
<container.page>
    <text>Profil<!text>
    <if.id == "42"><text>Utilisateur 42<!text><!if><else><text>Autre utilisateur<!text><!else>
    <if.payload == "depuis-menu"><text>Ouvert depuis le menu<!text><!if>
<!container>
