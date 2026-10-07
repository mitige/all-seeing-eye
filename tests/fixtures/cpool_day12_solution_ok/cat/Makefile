##
## EPITECH PROJECT, 2026
## cpool_day12
## File description:
## cat
##

CC	=	cc
CFLAGS	=	-Wall -Wextra

NAME	=	cat
SRC	=	cat.c
OBJ	=	$(SRC:.c=.o)

all:	$(NAME)

$(NAME):	$(OBJ)
	$(CC) $(CFLAGS) -o $(NAME) $(OBJ)

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)

re:	fclean all

.PHONY:	all clean fclean re
