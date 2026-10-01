#!/bin/bash
# Verification de bout en bout d'Azure, avec les vrais binaires : Azure est
# installe dans un dossier temporaire (jamais dans ~/.local), les daemons
# sont lances par azure-provider, et deux apps enfermees (examples/e2e_*)
# utilisent stockage, flux persistant, appels, messages en attente et
# evenements, et tentent de sortir de leur bac a sable.
#   scripts/verification-e2e.sh        (apres cargo build --workspace --bins --examples)
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
T="$ROOT/target/debug"
# Pas dans /tmp : le bac a sable laisse /tmp accessible, alors qu'un vrai
# dossier personnel ne l'est pas.
mkdir -p "$ROOT/target/e2e"
H="$(mktemp -d "$ROOT/target/e2e/home-XXXXXX")"
export HOME="$H" XDG_DATA_HOME="$H/share" XDG_CONFIG_HOME="$H/config" XDG_STATE_HOME="$H/state"
BIN="$H/share/azure/bin"
LOGS="$H/state/azure/apps"
PASS=0; FAIL=0
ok()   { echo "  OK    $1"; PASS=$((PASS+1)); }
ko()   { echo "  ECHEC $1"; FAIL=$((FAIL+1)); }
check() { if grep -q -- "$2" "$3" 2>/dev/null; then ok "$1"; else ko "$1 (attendu : $2)"; fi; }
nocheck() { if grep -q -- "$2" "$3" 2>/dev/null; then ko "$1 (trouve : $2)"; else ok "$1"; fi; }

# Sockets des daemons dans un dossier a part : la verification ne touche
# pas aux daemons qui tourneraient deja.
export AZURE_RUNTIME_DIR="$H/run"

echo "== Installation (dans $H)"
"$T/azure" setup --from "$T" --sans-manager > "$H/setup.log" 2>&1 && ok "azure setup" || ko "azure setup"
( setsid "$BIN/azure_provider" > "$H/provider.log" 2>&1 & )
sleep 3
"$BIN/azure_provider" status > "$H/status.log" 2>&1
for d in rooter stockage service manager; do check "daemon $d actif" "^$d .*actif" "$H/status.log"; done
for d in stockage_daemon service_daemon routeur_daemon azure_provider; do
    p=$(pgrep -f "^$BIN/$d" | head -1)
    if [ -n "$p" ] && ! cat /proc/$p/environ > /dev/null 2>&1; then ok "$d : memoire illisible"; else ko "$d : memoire lisible"; fi
done
"$BIN/azure" install "$ROOT/azure-foundation/examples/e2e/boutique" --bin "$T/examples/e2e_boutique" > "$H/install.log" 2>&1
"$BIN/azure" install "$ROOT/azure-foundation/examples/e2e/caisse" --bin "$T/examples/e2e_caisse" >> "$H/install.log" 2>&1
check "boutique installee, identite enregistree" "identite enregistree" "$H/install.log"
[ "$(grep -c 'identite enregistree' "$H/install.log")" = 2 ] && ok "caisse installee, identite enregistree" || ko "caisse installee"

check "taches de fond de la boutique enregistrees a l'installation" "^\[e2e-boutique-reveil\]" "$H/share/azure/services/e2e-boutique.conf"
check "taches de fond declarees au provider" "taches de fond declarees a azure-provider : e2e-boutique-reveil" "$H/install.log"

echo "== Reveil (la boutique n'a jamais ete ouverte)"
"$BIN/azure" run e2e-caisse reveil > /dev/null
for i in $(seq 1 40); do grep -q "E2E caisse \[reveil\] fin" "$LOGS/e2e-caisse.log" 2>/dev/null && break; sleep 0.5; done
check "appel qui reveille l'app fermee" "E2E caisse \[reveil\] taxe=20 en" "$LOGS/e2e-caisse.log"
check "second appel servi aussitot" "E2E caisse \[reveil\] taxe-encore=20" "$LOGS/e2e-caisse.log"
check "tache de fond lancee par le provider, enfermee" "E2E boutique reveillee sert-taxe=true enfermee=true" "$H/state/azure/provider/e2e-boutique-reveil.log"
"$BIN/azure_provider" status > "$H/status-reveil.log" 2>&1
check "tache de fond active dans le provider" "^e2e-boutique-reveil .*actif" "$H/status-reveil.log"

echo "== Scenario"
"$BIN/azure" run e2e-caisse phase1 > /dev/null
sleep 2
"$BIN/azure" run e2e-boutique 12 > /dev/null
for i in $(seq 1 40); do grep -q "E2E caisse \[phase1\] fin" "$LOGS/e2e-caisse.log" 2>/dev/null && break; sleep 0.5; done
sleep 1
C="$LOGS/e2e-caisse.log"; B="$LOGS/e2e-boutique.log"
check "caisse enfermee" "E2E caisse \[phase1\] id=[0-9]* enfermee=true" "$C"
check "boutique installee et enfermee" "E2E boutique enfermee=true" "$B"
check "message envoye a une app fermee" "E2E caisse message envoye" "$C"
check "appel refuse tant que la boutique ne tourne pas" "prix-avant-erreur=.*pas disponible" "$C"
check "message livre au lancement de la boutique" "E2E boutique message=bonjour de la caisse" "$B"
check "stockage de la boutique" "E2E boutique stockage lancements=1" "$B"
check "flux persistant partage" "E2E boutique flux panier seq=2" "$B"
check "methode servie" "E2E boutique sert prix=true" "$B"
check "flux recu par la caisse" 'panier={"items": \["pomme"\], "total": 42}' "$C"
check "appel avec reponse" "E2E caisse prix=42" "$C"
check "erreur de l'app appelee transmise" "prix-sans-produit-erreur=produit attendu" "$C"
check "bac a sable : cle maitre illisible" "cle-maitre-lisible=false" "$B"
check "bac a sable : config d'Azure non modifiable" "config-modifiable=false" "$B"
check "bac a sable : reseau coupe" "E2E boutique reseau=false" "$B"
check "bac a sable : dossier d'une autre app illisible" "dossier-voisin-lisible=false" "$C"
if grep -q "bus-joignable" "$B"; then check "bac a sable : bus D-Bus de session hors d'atteinte" "bus-joignable=false" "$B"; fi
check "bac a sable : X11 hors d'atteinte" "x11-joignable=false" "$B"
check "isolation : l'app est seule dans son espace de processus" "pid-isole=true" "$B"
check "bac a sable : le provider ne lance rien pour l'app" "provider-pilotable=false" "$B"
check "bac a sable : aucun signal aux daemons" "daemons=[1-9][0-9]* signal-daemon=false" "$B"
nocheck "aucune erreur de demarrage" "ERREUR" "$C"
nocheck "aucune erreur dans la boutique" "ERREUR" "$B"

echo "== Tableau de bord (azure_manager)"
"$BIN/azure_manager" apps > "$H/apps.log" 2>&1
check "apps connues du manager" "e2e-boutique" "$H/apps.log"
"$BIN/azure_manager" liens > "$H/liens.log" 2>&1
check "lien flux autorise" "e2e-caisse écoute « panier » de e2e-boutique : autorisé" "$H/liens.log"
check "lien appel autorise" "e2e-caisse appelle « prix » de e2e-boutique : autorisé" "$H/liens.log"
"$BIN/azure_manager" evenements > "$H/events.log" 2>&1
check "evenement d'erreur remonte" "erreur de test (volontaire)" "$H/events.log"
check "enfermement signale" "enfermee" "$H/events.log"

echo "== Identite"
cp "$H/share/azure/apps/e2e-caisse/e2e_caisse" "$H/caisse-deplacee"
cp "$H/share/azure/apps/e2e-caisse/app.azure" "$H/app.azure"
"$H/caisse-deplacee" verif > "$H/deplacee.log" 2>&1 &
sleep 4; kill %1 2>/dev/null
check "app deplacee reconnue (meme empreinte)" "E2E caisse \[verif\] id=" "$H/deplacee.log"
cp "$H/caisse-deplacee" "$H/caisse-modifiee"; printf 'x' >> "$H/caisse-modifiee"
"$H/caisse-modifiee" verif > "$H/modifiee.log" 2>&1
check "binaire modifie refuse" "n'est pas le sien" "$H/modifiee.log"

echo "== Redemarrage d'azure-service (flux persistant)"
sleep 12   # la boutique se termine
kill -9 "$(pgrep -f "^$BIN/service_daemon")" 2>/dev/null
sleep 3
"$BIN/azure_provider" status > "$H/status2.log" 2>&1
check "azure-service relance par le provider" "^service .*actif .* 1 " "$H/status2.log"
"$BIN/azure" run e2e-caisse verif > /dev/null
for i in $(seq 1 40); do grep -q "E2E caisse \[verif\] fin" "$C" 2>/dev/null && break; sleep 0.5; done
check "panier retrouve apres redemarrage" 'E2E caisse \[verif\] panier={"items": \["pomme"\], "total": 42}' "$C"
[ "$(stat -c %a "$AZURE_RUNTIME_DIR")" = 700 ] && ok "dossier des sockets prive (700)" || ko "droits du dossier des sockets"
[ "$(stat -c %a "$H/share/azure/service")" = 700 ] && ok "dossier d'azure-service prive (700)" || ko "droits du dossier d'azure-service"
if head -c 4 "$H/share/azure/service/flux/"*-panier.flux 2>/dev/null | grep -q AZS1; then ok "flux persistant chiffre sur disque"; else ko "flux persistant chiffre"; fi
if head -c 4 "$H/share/azure/manager/manager.bin" 2>/dev/null | grep -q AZS1; then ok "registre du manager chiffre"; else ko "registre du manager chiffre"; fi

echo "== Arret"
"$BIN/azure_provider" shutdown > /dev/null 2>&1
sleep 2
left=$(pgrep -f "^$BIN/(stockage_daemon|service_daemon|routeur_daemon|manager_daemon|azure_provider)" | wc -l)
[ "$left" = 0 ] && ok "tout s'arrete proprement" || ko "$left daemon(s) encore lances"

echo
echo "Resultat : $PASS OK, $FAIL echec(s). Journaux : $H"
[ "$FAIL" = 0 ]
