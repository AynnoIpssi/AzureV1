<container.az-fieldset.{{class}}#{{id}}>
    <if.titre != ""><text.az-fieldset-title>{{titre}}<!text><!if>
    <if.description != ""><text.az-fieldset-desc>{{description}}<!text><!if>
    <container.az-fieldset-body><slot/><!container>
<!container>
