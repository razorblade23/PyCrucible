import pygame, sys
pygame.init()
screen = pygame.display.set_mode((300, 200))
pygame.display.set_caption("PyCrucible GUI Benchmark")
screen.fill((40, 60, 200))
pygame.display.flip()
pygame.time.wait(1000)
pygame.quit()
