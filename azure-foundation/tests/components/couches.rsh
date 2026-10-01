<container.p>
<input#nom placeholder="Nom"/>
<checkbox#cgu>J'accepte<!checkbox>
<row>
    <radio#r1 name="t" value="S" checked="true">S<!radio>
    <radio#r2 name="t" value="M">M<!radio>
    <radio#r3 name="t" value="L">L<!radio>
<!row>
<slider#vol value="50" step="10"/>
<select#pays options="France, Belgique, Suisse"/>
<tooltip texte="Enregistre le formulaire"><btn.primary#ok>OK<!btn><!tooltip>
<container.liste>
    <for.n in nombres><btn#ligne-{{n}}>Ligne {{n}}<!btn><!for>
<!container>
<if.modale == true>
<modal titre="Confirmer" ouvert="true" id="m">
    <text>Supprimer ce fichier ?<!text>
    <input#raison placeholder="Pourquoi ?"/>
    <row><btn.danger#m-oui>Supprimer<!btn><btn#m-annuler>Annuler<!btn><!row>
<!modal>
<!if>
<toasts>
    <toast.succes titre="Enregistre">Tout est a jour.<!toast>
<!toasts>
<!container>
