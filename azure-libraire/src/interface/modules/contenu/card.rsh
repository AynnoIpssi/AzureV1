<container.az-card.{{class}}#{{id}}>
    <if.titre != ""><title3.az-card-title>{{titre}}<!title3><!if>
    <if.description != ""><text.az-card-desc>{{description}}<!text><!if>
    <if.a_contenu == true><container.az-card-body><slot/><!container><!if>
    <if.pied != ""><text.az-card-foot>{{pied}}<!text><!if>
<!container>
