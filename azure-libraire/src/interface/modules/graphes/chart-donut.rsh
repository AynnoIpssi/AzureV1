<container.az-chart.{{class}}#{{id}}>
    <if.titre != ""><container.az-chart-head>
        <container.az-chart-main>
            <text.az-chart-title>{{titre}}<!text>
            <if.detail != ""><text.az-chart-detail>{{detail}}<!text><!if>
        <!container>
        <if.valeur != ""><text.az-chart-value>{{valeur}}<!text><!if>
    <!container><!if>
    <graphe.az-chart-plot.az-plot-anneau type="anneau" valeurs="{{valeurs}}" series="{{series}}" etiquettes="{{etiquettes}}" min="{{min}}" max="{{max}}" unite="{{unite}}" legende="{{legende}}" grille="{{grille}}" lisse="{{lisse}}" centre="{{centre}}" couleurs="{{couleurs}}"/>
    <if.aide != ""><text.az-chart-help>{{aide}}<!text><!if>
<!container>
