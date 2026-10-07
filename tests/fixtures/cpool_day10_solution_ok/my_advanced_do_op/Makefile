##
## EPITECH PROJECT, 2026
## cpool_day10
## File description:
## Makefile for my_advanced_do-op
##

CC	=	gcc

CFLAGS	=	-Wall -Wextra

NAME	=	my_advanced_do-op

SRC	=	my_advanced_do_op.c	\
		my_add.c	\
		my_sub.c	\
		my_mul.c	\
		my_div.c	\
		my_mod.c	\
		my_usage.c

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
