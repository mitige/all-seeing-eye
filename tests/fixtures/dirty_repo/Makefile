##
## EPITECH PROJECT, 2026
## dirty_repo fixture
## File description:
## Makefile sans regle re
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

.PHONY:	all clean fclean
