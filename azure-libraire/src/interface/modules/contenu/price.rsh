<container.az-price.{{class}}#{{id}}>
    <text.az-price-name>{{nom}}<!text>
    <container.az-price-row>
        <text.az-price-amount>{{prix}}<!text>
        <text.az-price-period>{{periode}}<!text>
    <!container>
    <for.item in items_list><text.az-price-item>+ {{item}}<!text><!for>
    <slot/>
<!container>
