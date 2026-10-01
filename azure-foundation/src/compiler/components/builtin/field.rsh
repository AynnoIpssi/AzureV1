<container.az-field.{{class}}>
    <if.label != ""><text.az-field-label>{{label}}<!text><!if>
    <slot/>
    <if.erreur != ""><text.az-field-error>{{erreur}}<!text><!if>
    <elseif.aide != ""><text.az-field-help>{{aide}}<!text><!elseif>
<!container>
