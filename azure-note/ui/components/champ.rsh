<!-- La valeur d'une propriete `p` (voir `page::propriete`), selon son type.
     Selection, etiquettes, date et relation s'ouvrent au clic (`ouvrir-`)
     sur un choix, sous la ligne (components/editeur-valeur.rsh). -->
<if.p.genre == "case"><checkbox.ch-case#{{p.fid}} checked="{{p.coche}}"/><!if>
<elseif.p.genre == "formule">
    <if.p.erreur == true><text.ch-calc.ch-erreur>{{p.texte}}<!text><!if>
    <else><text.ch-calc>{{p.texte}}<!text><!else>
<!elseif>
<elseif.p.genre == "texte"><input.ch-texte#{{p.fid}} value="{{p.valeur}}" placeholder="Vide"/><!elseif>
<elseif.p.genre == "nombre"><input.ch-texte.ch-nombre#{{p.fid}} value="{{p.valeur}}" placeholder="Vide"/><!elseif>
<elseif.p.vide == true><button.ch-bouton.ch-vide.{{p.ouvert}}#ouvrir-{{p.fid}}>Vide<!button><!elseif>
<elseif.p.genre == "date"><button.ch-bouton.{{p.ouvert}}#ouvrir-{{p.fid}}>{{p.date}}<!button><!elseif>
<else>
    <container.ch-pastilles>
    <for.x in p.pastilles><button.opt.{{x.teinte}}#ouvrir-{{p.fid}}>{{x.nom}}<!button><!for>
    <!container>
<!else>
