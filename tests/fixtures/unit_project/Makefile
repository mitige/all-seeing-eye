##
## EPITECH PROJECT, 2026
## unit_project fixture
## File description:
## Makefile
##

NAME	=	prog

SRC	=	main.c

OBJ	=	$(SRC:.c=.o)

CC	=	cc
CFLAGS	=	-Wall -Wextra -Werror

all:	$(NAME)

$(NAME):	$(OBJ)
	$(CC) -o $(NAME) $(OBJ)

## tests_run : faux binaire criterion — un script déterministe suffit
## (Task 8 : c'est le parsing de la sortie qui compte, pas la
## compilation d'un vrai binaire criterion). Les scénarios KO/ANSI/
## vide sont écrits par tests/unit.rs par-dessus la copie en salle
## blanche.
tests_run:
	@./tests_run.sh

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)

re:	fclean all

.PHONY:	all clean fclean re tests_run
