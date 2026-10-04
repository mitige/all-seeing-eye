##
## EPITECH PROJECT, 2026
## slow_makefile fixture
## File description:
## Makefile dont l evaluation bloque (sleep dans $(shell))
##

DUMMY := $(shell sleep 60)

all:
	@echo hello

clean:
	rm -f hello.o

fclean: clean
	rm -f hello

re: fclean all

.PHONY: all clean fclean re
