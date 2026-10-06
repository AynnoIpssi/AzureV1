<container.az-stat.az-stat-spark.{{class}}#{{id}}>
    <text.az-stat-label>{{label}}<!text>
    <text.az-stat-value>{{valeur}}<!text>
    <if.detail != ""><text.az-stat-detail.{{tendance}}>{{detail}}<!text><!if>
    <if.ton != ""><graphe.az-spark.{{ton}} type="spark" valeurs="{{valeurs}}" min="{{min}}" max="{{max}}"/><!if>
    <else><graphe.az-spark type="spark" valeurs="{{valeurs}}" min="{{min}}" max="{{max}}"/><!else>
<!container>
