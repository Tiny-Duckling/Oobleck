from django.db import models

from .create_update_model import CreateUpdateModel


class Level(CreateUpdateModel):
    name = models.CharField(max_length=200)
    description = models.TextField()

    def __str__(self):
        return self.name
