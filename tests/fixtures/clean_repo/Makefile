##
## EPITECH PROJECT, 2026
## clean_repo fixture
## File description:
## Makefile
##

NAME	=	hello

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
