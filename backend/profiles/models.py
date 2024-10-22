from django.contrib.auth.models import AbstractUser
from django.db import models


class User(AbstractUser):
    credits = models.IntegerField(default=120)

    def __str__(self):
        return self.username
