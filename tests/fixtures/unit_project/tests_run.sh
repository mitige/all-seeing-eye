#!/bin/sh
# Faux tests_run au format criterion (fixture unit_project, Task 8) :
# sortie déterministe, trois tests OK. Deterministe et sans dépendance
# — les scénarios KO/ANSI/vide sont écrits par tests/unit.rs par-dessus
# ce script dans la copie en salle blanche.
echo 'tests::my_strlen: [OK]'
echo 'tests::my_strcpy: [OK]'
echo 'tests::my_strdup: [OK]'
echo 'Tests: 3 | Passing: 3 | Failing: 0'
exit 0
