<container.az-setting.{{class}}#{{id}}>
    <container.az-setting-main>
        <text.az-setting-title>{{titre}}<!text>
        <if.description != ""><text.az-setting-desc>{{description}}<!text><!if>
    <!container>
    <container.az-setting-control><slot/><!container>
<!container>
