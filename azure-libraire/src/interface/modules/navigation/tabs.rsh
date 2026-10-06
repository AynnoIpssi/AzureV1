<container.az-tabs.{{class}}>
<for.onglet in items_list>
    <if.onglet == actif><button.az-tab.on#{{id}}-{{onglet_index}}>{{onglet}}<!button><!if>
    <else><button.az-tab#{{id}}-{{onglet_index}}>{{onglet}}<!button><!else>
<!for>
<!container>
