from rest_framework import viewsets

from .mixins import AutoUserMixin
from .models import Opening
from .serializers import OpeningSerializer

from backend.permissions import IsStaffOrReadOnly


class OpeningViewSet(AutoUserMixin, viewsets.ModelViewSet):
    queryset = Opening.objects.filter(Q(date=now.date(),time__gte=now.time()|Q(date__gt=now.date())).order_by('-date')
    serializer_class = OpeningSerializer
    permission_classes = [IsStaffOrReadOnly]


