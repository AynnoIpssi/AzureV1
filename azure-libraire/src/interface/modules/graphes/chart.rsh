<container.az-chart.{{class}}#{{id}}>
    <if.titre != ""><container.az-chart-head>
        <container.az-chart-main>
            <text.az-chart-title>{{titre}}<!text>
            <if.detail != ""><text.az-chart-detail>{{detail}}<!text><!if>
        <!container>
        <if.valeur != ""><text.az-chart-value>{{valeur}}<!text><!if>
    <!container><!if>
    <slot/>
    <if.aide != ""><text.az-chart-help>{{aide}}<!text><!if>
<!container>
