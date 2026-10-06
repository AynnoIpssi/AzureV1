<container.az-page.{{class}}#{{id}}>
    <if.titre != ""><title1.az-page-title>{{titre}}<!title1><!if>
    <if.description != ""><text.az-page-desc>{{description}}<!text><!if>
    <slot/>
<!container>
