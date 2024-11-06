from django.db import models

from .category import Category
from .create_update_model import CreateUpdateModel


class Topic(CreateUpdateModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    category = models.ForeignKey(Category, on_delete=models.PROTECT)

    def __str__(self):
        return self.name
