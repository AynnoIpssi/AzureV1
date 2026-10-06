<container.az-empty.{{class}}#{{id}}>
    <container.az-empty-icon><!container>
    <text.az-empty-title>{{titre}}<!text>
    <if.description != ""><text.az-empty-text>{{description}}<!text><!if>
    <slot/>
<!container>
