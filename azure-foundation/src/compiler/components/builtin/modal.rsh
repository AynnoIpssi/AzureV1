<if.ouvert == "true">
<container.az-modal-backdrop>
    <container.az-modal.{{class}}#{{id}}>
        <container.az-modal-head>
            <text.az-modal-title>{{titre}}<!text>
            <button.az-modal-close#{{id}}-fermer>x<!button>
        <!container>
        <container.az-modal-body><slot/><!container>
    <!container>
<!container>
<!if>
