<container.az-accordion.{{class}}>
    <if.ouvert == "true">
        <button.az-accordion-head.on#{{id}}>{{titre}}<!button>
        <container.az-accordion-body><slot/><!container>
    <!if>
    <else><button.az-accordion-head#{{id}}>{{titre}}<!button><!else>
<!container>
