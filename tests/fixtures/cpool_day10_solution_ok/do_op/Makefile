##
## EPITECH PROJECT, 2026
## cpool_day10
## File description:
## Makefile for do-op
##

CC	=	gcc

CFLAGS	=	-Wall -Wextra

NAME	=	do-op

SRC	=	do_op.c

OBJ	=	$(SRC:.c=.o)

all:	$(NAME)

$(NAME):	$(OBJ)
	$(CC) -o $(NAME) $(OBJ) -L../lib -lmy

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)

re:	fclean all

.PHONY:	all clean fclean re
