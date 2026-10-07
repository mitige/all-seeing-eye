/*
** EPITECH PROJECT, 2026
** cpool_day07
** File description:
** main de test pour les programmes a arguments (task04 a task06)
*/

/*
** Le programme etudiant livre son propre main (delivery dossier
** taskNN, tous ses .c) ; la batterie le renomme cpool_d07_student_main
** a la compile (link_flags "-Dmain=cpool_d07_student_main") et ce
** harness fabrique l'argv du sujet : ./a.out test "This is a test "
** retest. L'argv[0] reel est imprevisible en salle blanche (tempdir
** aleatoire), d'ou l'argv fabrique en dur plutot que le champ args.
*/
#undef main

extern int cpool_d07_student_main(int argc, char **argv);

int main(void)
{
    char *argv[5];

    argv[0] = "./a.out";
    argv[1] = "test";
    argv[2] = "This is a test ";
    argv[3] = "retest";
    argv[4] = (char *)0;
    return (cpool_d07_student_main(4, argv));
}
