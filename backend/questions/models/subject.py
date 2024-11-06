from django.db import models

from .level import Level
from .create_update_model import CreateUpdateModel


class Subject(CreateUpdateModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    level = models.ForeignKey(Level, on_delete=models.PROTECT)

    def __str__(self):
        return self.name
