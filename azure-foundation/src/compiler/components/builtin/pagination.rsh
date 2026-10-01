<container.az-pagination.{{class}}>
<for.p in pages_range>
    <if.p == page><button.az-pagelink.on#{{id}}-{{p}}>{{p}}<!button><!if>
    <else><button.az-pagelink#{{id}}-{{p}}>{{p}}<!button><!else>
<!for>
<!container>
