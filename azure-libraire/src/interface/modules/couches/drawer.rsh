<if.ouvert == "true">
<container.az-drawer-backdrop>
    <container.az-drawer.{{class}}#{{id}}>
        <container.az-modal-head>
            <text.az-modal-title>{{titre}}<!text>
            <button.az-modal-close#{{id}}-fermer>x<!button>
        <!container>
        <container.az-drawer-body><slot/><!container>
    <!container>
<!container>
<!if>
