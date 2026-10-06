<container.az-breadcrumb.{{class}}>
<for.item in items_list>
    <if.item_index != 0><text.az-crumb-sep>/<!text><!if>
    <text.az-crumb>{{item}}<!text>
<!for>
<!container>
