<if.ouvert == "true">
<container.az-modal-backdrop>
    <container.az-modal.az-confirm.{{class}}#{{id}}>
        <text.az-modal-title>{{titre}}<!text>
        <if.a_contenu == true><text.az-confirm-text>{{contenu}}<!text><!if>
        <container.az-actions>
            <if.non != ""><button.az-btn#{{id}}-fermer>{{non}}<!button><!if>
            <else><button.az-btn#{{id}}-fermer>Annuler<!button><!else>
            <if.oui != ""><button.az-btn.primary.az-confirm-oui#{{id}}-oui>{{oui}}<!button><!if>
            <else><button.az-btn.primary.az-confirm-oui#{{id}}-oui>Confirmer<!button><!else>
        <!container>
    <!container>
<!container>
<!if>
