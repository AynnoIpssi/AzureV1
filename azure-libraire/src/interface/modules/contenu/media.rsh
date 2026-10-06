<container.az-media.{{class}}#{{id}}>
    <if.nom != ""><avatar nom="{{nom}}"/><!if>
    <container.az-media-main>
        <text.az-media-title>{{titre}}<!text>
        <if.description != ""><text.az-media-desc>{{description}}<!text><!if>
    <!container>
    <slot/>
<!container>
