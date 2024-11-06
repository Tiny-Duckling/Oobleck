from django.db import models

from .question import Question


class Choice(models.Model):
    multi_choice_question = models.ForeignKey(
        Question, on_delete=models.PROTECT, related_name="choices"
    )
    choice_text = models.CharField(max_length=200)
    choice_image = models.ImageField(null=True, blank=True)
    is_correct = models.BooleanField(default=False)

    def __str__(self):
        return self.choice_text
