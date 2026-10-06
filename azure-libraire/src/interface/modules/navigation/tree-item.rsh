<if.actif == "true">
    <if.ouvert == "true"><button.az-tree-item.on.az-niveau-{{niveau}}.{{class}}#{{id}}>- {{contenu}}<!button><!if>
    <elseif.ouvert == "false"><button.az-tree-item.on.az-niveau-{{niveau}}.{{class}}#{{id}}>+ {{contenu}}<!button><!elseif>
    <else><button.az-tree-item.on.az-feuille.az-niveau-{{niveau}}.{{class}}#{{id}}>{{contenu}}<!button><!else>
<!if>
<else>
    <if.ouvert == "true"><button.az-tree-item.az-niveau-{{niveau}}.{{class}}#{{id}}>- {{contenu}}<!button><!if>
    <elseif.ouvert == "false"><button.az-tree-item.az-niveau-{{niveau}}.{{class}}#{{id}}>+ {{contenu}}<!button><!elseif>
    <else><button.az-tree-item.az-feuille.az-niveau-{{niveau}}.{{class}}#{{id}}>{{contenu}}<!button><!else>
<!else>
