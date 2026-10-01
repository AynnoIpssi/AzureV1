<container.hero>
    <title1>Mon Application<!title1>
    <text>Bienvenue sur ta page d'accueil.<!text>

    <if.userisadmin>
        <container.card>
            <title2>Panneau administrateur<!title2>
            <button.primary-button>Gerer les utilisateurs<!button>
            <button.primary-button>Voir les logs<!button>
        <!container>
    <!if>
    <elseif.userisguest>
        <text.muted-text>Tu navigues en tant qu'invite.<!text>
    <!elseif>
    <else>
        <text.muted-text>Connecte-toi pour acceder a plus de fonctionnalites.<!text>
    <!else>

    <while.has_next_notification()>
        <text>Tu as une nouvelle notification.<!text>
    <!while>

    <for.item in liste_items>
        <text>Element de la liste<!text>
    <!for>

    <container.card>
        <title3>Galerie<!title3>
        <image><!image>
        <video><!video>
    <!container>
<!container>

<container>
    <title>Test<!title>
    <container>
        <title>Test<!title>
    <!container>
<!container>
