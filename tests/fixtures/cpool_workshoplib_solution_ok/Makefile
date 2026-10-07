##
## EPITECH PROJECT, 2026
## cpool_workshoplib
## File description:
## libmy.a
##

CC	=	cc
CFLAGS	=	-Wall -Wextra -Iinclude
AR	=	ar rc

NAME	=	lib/libmy.a
SRC	=	$(wildcard lib/my/*.c)
OBJ	=	$(SRC:.c=.o)

all:	$(NAME)

$(NAME):	$(OBJ)
	$(AR) $(NAME) $(OBJ)
	ranlib $(NAME)

clean:
	rm -f $(OBJ)

fclean:	clean
	rm -f $(NAME)

re:	fclean all

.PHONY:	all clean fclean re
