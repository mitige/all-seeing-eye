##
## EPITECH PROJECT, 2026
## warn_project fixture
## File description:
## Makefile
##

NAME	=	warn_project

SRC	=	main.c

OBJ	=	$(SRC:.c=.o)

CC	=	cc
CFLAGS	=	-Wall -Wextra -Werror

all:	$(NAME)

$(NAME):	$(OBJ)
	$(CC) -o $(NAME) $(OBJ)

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)

re:	fclean all

.PHONY:	all clean fclean re
