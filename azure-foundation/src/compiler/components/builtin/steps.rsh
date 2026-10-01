<container.az-steps.{{class}}>
<for.etape in items_list>
    <container.az-step>
        <if.etape_rang \< courant><text.az-step-dot.done>{{etape_rang}}<!text><!if>
        <elseif.etape_rang == courant><text.az-step-dot.current>{{etape_rang}}<!text><!elseif>
        <else><text.az-step-dot>{{etape_rang}}<!text><!else>
        <text.az-step-label>{{etape}}<!text>
    <!container>
<!for>
<!container>
