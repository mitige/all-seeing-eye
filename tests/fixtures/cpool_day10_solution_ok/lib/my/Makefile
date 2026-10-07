##
## EPITECH PROJECT, 2026
## cpool_day10
## File description:
## Makefile for libmy
##

CC	=	gcc

CFLAGS	=	-Wall -Wextra

NAME	=	libmy.a

SRC	=	my_putchar.c	\
		my_putstr.c	\
		my_put_nbr.c	\
		my_getnbr.c	\
		my_strlen.c

OBJ	=	$(SRC:.c=.o)

all:	$(NAME)

$(NAME):	$(OBJ)
	ar rcs $(NAME) $(OBJ)
	mkdir -p ../../include
	cp $(NAME) ..
	cp my.h ../../include

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)
	rm -f ../$(NAME)
	rm -f ../../include/my.h

re:	fclean all

.PHONY:	all clean fclean re
